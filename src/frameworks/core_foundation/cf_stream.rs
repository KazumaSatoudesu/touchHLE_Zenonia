/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFReadStream`.

use super::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use super::cf_url::CFURLRef;
use super::CFIndex;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::mem::MutPtr;
use crate::objc::{id, msg, msg_class, objc_classes, ClassExports, HostObject, NSZonePtr};
use crate::Environment;
use crate::fs::GuestFile;

pub type CFReadStreamRef = super::CFTypeRef;

/// Status constants matching Apple's CFStreamStatus enum.
#[allow(dead_code)]
const kCFStreamStatusNotOpen: i32 = 0;
const kCFStreamStatusOpen: i32 = 2;
const kCFStreamStatusError: i32 = 5;

/// Host object stored behind every `_touchHLE_CFReadStream` instance.
struct CFReadStreamHostObject {
    path: String,
    file: Option<GuestFile>,
    file_size: usize,   // total size of the file
    bytes_read: usize,  // how many bytes read so far
}

impl HostObject for CFReadStreamHostObject {}

pub fn CFReadStreamCreateWithFile(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    file_url: CFURLRef,
) -> CFReadStreamRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default());

    let path_ns: id = msg![env; file_url path];
    let _keep_alive: id = msg![env; path_ns retain];
    let path = to_rust_string(env, path_ns);

    let host_object = Box::new(CFReadStreamHostObject {
        path: path.to_string(),
        file: None,
        file_size: 0,
        bytes_read: 0,
    });

    let class = env
        .objc
        .get_known_class("_touchHLE_CFReadStream", &mut env.mem);
    let stream = env.objc.alloc_object(class, host_object, &mut env.mem);

    let result = crate::objc::retain(env, stream);
    log!("CFReadStreamCreateWithFile('{}') -> {:?}", path, result);
    result
}

fn CFReadStreamOpen(env: &mut Environment, stream: CFReadStreamRef) -> bool {
    log!("CFReadStreamOpen called, stream={:?}", stream);
    let host_obj = env.objc.borrow_mut::<CFReadStreamHostObject>(stream);
    if host_obj.file.is_some() {
        return true;
    }
    let path = host_obj.path.clone();

    // Empty path = dummy stream, nothing to open
    if path.is_empty() {
        log!("CFReadStreamOpen: empty path, skipping");
        return false;
    }

    // Get file size first
    let file_size = match env.fs.read(crate::fs::GuestPath::new(&path)) {
        Ok(bytes) => bytes.len(),
        Err(()) => 0,
    };

    let guest_path = crate::fs::GuestPath::new(&path);
    match env.fs.open(guest_path) {
        Ok(guest_file) => {
            let host_obj = env.objc.borrow_mut::<CFReadStreamHostObject>(stream);
            log!("CFReadStreamOpen('{}') => success, size={}", path, file_size);
            host_obj.file = Some(guest_file);
            host_obj.file_size = file_size;
            host_obj.bytes_read = 0;
            true
        }
        Err(()) => {
            log!("CFReadStreamOpen('{}') failed: not found in guest fs", path);
            false
        }
    }
}

fn CFReadStreamRead(
    env: &mut Environment,
    stream: CFReadStreamRef,
    buffer: MutPtr<u8>,
    buffer_length: CFIndex,
) -> CFIndex {
    use std::io::Read;
    log!("CFReadStreamRead called, stream={:?}, length={}", stream, buffer_length);
    let buf_len: usize = buffer_length.try_into().unwrap();
    let host_obj = env.objc.borrow_mut::<CFReadStreamHostObject>(stream);

    // Empty path = dummy stream, return EOF
    if host_obj.path.is_empty() {
        log!("CFReadStreamRead: empty path stream, returning EOF");
        return 0;
    }

    let Some(file) = host_obj.file.as_mut() else {
        log!("CFReadStreamRead: stream is not open");
        return -1;
    };

    // Read into a host-side buffer first, then copy into guest memory.
    let mut tmp = vec![0u8; buf_len];
    match file.read(&mut tmp) {
        Ok(n) => {
            if n == 0 {
                log!("CFReadStreamRead: EOF");
                return 0;
            }
            let dest = env.mem.bytes_at_mut(buffer, n.try_into().unwrap());
            dest.copy_from_slice(&tmp[..n]);
            host_obj.bytes_read += n;
            log!(
                "CFReadStreamRead: read {} bytes ({}/{})",
                n, host_obj.bytes_read, host_obj.file_size
            );
            n as CFIndex
        }
        Err(e) => {
            log!("CFReadStreamRead error: {}", e);
            -1
        }
    }
}

fn CFReadStreamClose(env: &mut Environment, stream: CFReadStreamRef) {
    let host_obj = env.objc.borrow_mut::<CFReadStreamHostObject>(stream);
    log_dbg!("CFReadStreamClose('{}')", host_obj.path);
    host_obj.file = None;
    host_obj.bytes_read = 0;
}

fn CFReadStreamGetStatus(env: &mut Environment, stream: CFReadStreamRef) -> i32 {
    let host_obj = env.objc.borrow::<CFReadStreamHostObject>(stream);
    if host_obj.file.is_some() {
        kCFStreamStatusOpen
    } else {
        kCFStreamStatusNotOpen
    }
}

fn CFReadStreamCopyProperty(
    _env: &mut Environment,
    _stream: CFReadStreamRef,
    _property_name: id,
) -> id {
    log!("TODO: CFReadStreamCopyProperty");
    id::null()
}

fn CFReadStreamGetError(
    _env: &mut Environment,
    _stream: CFReadStreamRef,
) -> id {
    log!("TODO: CFReadStreamGetError");
    id::null()
}

fn CFURLCreatePropertyFromResource(
    env: &mut Environment,
    url: id,
    _property: id,
    _error_code: id,
) -> id {
    log!("CFURLCreatePropertyFromResource CALLED url={:?}", url);
    use crate::objc::nil;
    if url == nil {
        // Return a large default so the game can allocate a buffer big enough
        // for any asset (Title.pzx is ~196KB so 256KB covers it safely).
        log!("CFURLCreatePropertyFromResource: nil url, returning large default size (262144)");
        let ns_number: id = msg_class![env; NSNumber numberWithInt:262144i32];
        return msg![env; ns_number retain];
    }
    let path: id = msg![env; url path];
    if path == nil {
        log!("CFURLCreatePropertyFromResource: nil path, returning 0");
        let ns_number: id = msg_class![env; NSNumber numberWithInt:0i32];
        return msg![env; ns_number retain];
    }
    let path_str = to_rust_string(env, path);
    let file_size: i32 = match env.fs.read(crate::fs::GuestPath::new(&*path_str)) {
        Ok(bytes) => bytes.len() as i32,
        Err(()) => {
            log!("CFURLCreatePropertyFromResource: could not read {}", path_str);
            0
        }
    };
    log!("CFURLCreatePropertyFromResource: size={} for {}", file_size, path_str);
    let ns_number: id = msg_class![env; NSNumber numberWithInt:file_size];
    msg![env; ns_number retain]
}

fn CFReadStreamHasBytesAvailable(env: &mut Environment, stream: CFReadStreamRef) -> bool {
    let host_obj = env.objc.borrow::<CFReadStreamHostObject>(stream);

    // Empty path = dummy stream, no bytes available
    if host_obj.path.is_empty() {
        log!("CFReadStreamHasBytesAvailable: empty path stream -> false");
        return false;
    }

    if host_obj.file.is_none() {
        log!("CFReadStreamHasBytesAvailable({:?}) -> false (not open)", stream);
        return false;
    }

    let available = host_obj.bytes_read < host_obj.file_size;
    log!(
        "CFReadStreamHasBytesAvailable({:?}) -> {} ({}/{})",
        stream, available, host_obj.bytes_read, host_obj.file_size
    );
    available
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFReadStreamCreateWithFile(_, _)),
    export_c_func!(CFReadStreamOpen(_)),
    export_c_func!(CFReadStreamRead(_, _, _)),
    export_c_func!(CFReadStreamClose(_)),
    export_c_func!(CFReadStreamGetStatus(_)),
    export_c_func!(CFReadStreamCopyProperty(_, _)),
    export_c_func!(CFReadStreamGetError(_)),
    export_c_func!(CFURLCreatePropertyFromResource(_, _, _)),
    export_c_func!(CFReadStreamHasBytesAvailable(_)),
];

/// The ObjC class backing CFReadStream objects.
/// We need this so that CFRelease / retain-counting works correctly
/// (CFRelease calls [obj release] under the hood in touchHLE).
pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_CFReadStream: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(CFReadStreamHostObject {
        path: String::new(),
        file: None,
        file_size: 0,
        bytes_read: 0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// CFRelease will call [stream release] which calls dealloc when rc hits 0.
// The default NSObject dealloc is fine — our HostObject Drop will clean up.

@end

};