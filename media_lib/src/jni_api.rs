//! JNI adapter for `org.anon.mediaplayer.MediaLib`.
//!
//! Only marshals values between the JVM and [`MediaService`]; every failure
//! is thrown as `org.anon.mediaplayer.MediaLibException(int code, String
//! message)`, with `code` from [`MediaLibError::code`].

#![allow(non_snake_case)]

use std::fs::File;
use std::os::fd::{FromRawFd, OwnedFd};
use std::path::{Path, PathBuf};

use jni::JNIEnv;
use jni::objects::{JByteArray, JClass, JString, JThrowable, JValue};
use jni::sys::{jbyteArray, jint, jlong};

use crate::crypto::wipe::wipe;
use crate::error::{MediaLibError, MediaResult};
use crate::service::MediaService;

/// JNI name of the Kotlin exception every native failure is thrown as.
const MEDIA_LIB_EXCEPTION: &str = "org/anon/mediaplayer/MediaLibException";

// ---------------------------------------------------------------------------
// Error and value marshalling
// ---------------------------------------------------------------------------

/// Runs `body`; on error throws it to Java and returns `fallback`.
fn throwing<'local, T>(
    env: &mut JNIEnv<'local>,
    fallback: T,
    body: impl FnOnce(&mut JNIEnv<'local>) -> MediaResult<T>,
) -> T {
    match body(env) {
        Ok(value) => value,
        Err(error) => {
            throw_media_error(env, &error);
            fallback
        }
    }
}

/// Throws `error` as a `MediaLibException`, falling back to a plain
/// `RuntimeException` if the exception class can't be constructed.
fn throw_media_error(env: &mut JNIEnv, error: &MediaLibError) {
    // A failed JNI call may have left an exception pending; clear it first.
    let _ = env.exception_clear();
    if try_throw_media_error(env, error).is_err() {
        let _ = env.exception_clear();
        let _ = env.throw_new("java/lang/RuntimeException", error.to_string());
    }
}

fn try_throw_media_error(env: &mut JNIEnv, error: &MediaLibError) -> jni::errors::Result<()> {
    let message = env.new_string(error.to_string())?;
    let exception = env.new_object(
        MEDIA_LIB_EXCEPTION,
        "(ILjava/lang/String;)V",
        &[JValue::Int(error.code()), JValue::Object(&message)],
    )?;
    env.throw(JThrowable::from(exception))
}

fn string(env: &mut JNIEnv, value: &JString) -> MediaResult<String> {
    Ok(env.get_string(value)?.into())
}

fn path(env: &mut JNIEnv, dir: &JString, name: &JString) -> MediaResult<PathBuf> {
    Ok(Path::new(&string(env, dir)?).join(string(env, name)?))
}

/// Copies plaintext into a Java array and wipes the native copy.
fn plaintext_to_java<'local>(env: &mut JNIEnv<'local>, mut plaintext: Vec<u8>) -> MediaResult<JByteArray<'local>> {
    let array = env.byte_array_from_slice(&plaintext);
    wipe(&mut plaintext);
    Ok(array?)
}

/// Reads key bytes from Java, runs `use_key`, then wipes the native copy.
fn with_key<T>(env: &mut JNIEnv, key: &JByteArray, use_key: impl FnOnce(&[u8]) -> MediaResult<T>) -> MediaResult<T> {
    let mut bytes = env.convert_byte_array(key)?;
    let result = use_key(&bytes);
    wipe(&mut bytes);
    result
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// `external fun nativeStripAndEncrypt(fd: Int, mime: String, outputDir: String,
/// fileName: String): ByteArray`
///
/// Strips and encrypts the media behind `fd` to `outputDir/fileName` and
/// returns its new 32-byte key.
///
/// # Safety
/// `input_fd` must be a valid, open file descriptor whose ownership passes to
/// this call (Kotlin calls `detachFd()`); it is closed on return.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_org_anon_mediaplayer_MediaLib_nativeStripAndEncrypt<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    input_fd: jint,
    mime: JString<'local>,
    output_dir: JString<'local>,
    file_name: JString<'local>,
) -> jbyteArray {
    // Take ownership first so the fd is closed even if marshalling fails.
    let input = File::from(unsafe { OwnedFd::from_raw_fd(input_fd) });

    throwing(&mut env, std::ptr::null_mut(), |env| {
        let mime = string(env, &mime)?;
        let dir = string(env, &output_dir)?;
        let name = string(env, &file_name)?;
        let key = MediaService::default().encrypt_to_file(input, &mime, Path::new(&dir), &name)?;
        Ok(env.byte_array_from_slice(&key[..])?.into_raw())
    })
}

/// `external fun nativeDecrypt(inputDir: String, inputName: String,
/// key: ByteArray): ByteArray`
///
/// Decrypts a whole image file into memory.
#[unsafe(no_mangle)]
pub extern "C" fn Java_org_anon_mediaplayer_MediaLib_nativeDecrypt<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    input_dir: JString<'local>,
    input_name: JString<'local>,
    key: JByteArray<'local>,
) -> JByteArray<'local> {
    throwing(&mut env, JByteArray::default(), |env| {
        let path = path(env, &input_dir, &input_name)?;
        let image = with_key(env, &key, |key| MediaService::default().decrypt_image(&path, key))?;
        plaintext_to_java(env, image)
    })
}

/// `external fun nativeReadMedia(inputDir: String, inputName: String,
/// position: Long, length: Long, key: ByteArray): ByteArray`
///
/// Returns up to `length` plaintext bytes at `position` of an audio/video
/// file, decrypting only the chunks that cover that range.
#[unsafe(no_mangle)]
pub extern "C" fn Java_org_anon_mediaplayer_MediaLib_nativeReadMedia<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    input_dir: JString<'local>,
    input_name: JString<'local>,
    position: jlong,
    length: jlong,
    key: JByteArray<'local>,
) -> jbyteArray {
    throwing(&mut env, std::ptr::null_mut(), |env| {
        let path = path(env, &input_dir, &input_name)?;
        let range = with_key(env, &key, |key| MediaService::default().read_range(&path, position, length, key))?;
        Ok(plaintext_to_java(env, range)?.into_raw())
    })
}
