/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `netdb.h`
use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::libc::sys::socket::{sockaddr, AF_INET, SOCK_DGRAM, SOCK_STREAM};
use crate::mem::{guest_size_of, ConstPtr, GuestUSize, MutPtr, Ptr, SafeRead};
use crate::Environment;

const AI_PASSIVE: i32 = 0x1;
pub const IPPROTO_TCP: i32 = 6;
pub const IPPROTO_UDP: i32 = 17;
const EAI_FAIL: i32 = 4;

#[allow(non_camel_case_types)]
pub type socklen_t = u32;

/// Minimal hostent struct matching the iOS ABI layout.
/// h_addr_list is a null-terminated array of pointers to in_addr structs.
#[derive(Copy, Clone)]
#[repr(C, packed)]
#[allow(non_camel_case_types)]
struct hostent {
    h_name: MutPtr<u8>,
    h_aliases: MutPtr<MutPtr<u8>>,
    h_addrtype: i32,
    h_length: i32,
    h_addr_list: MutPtr<MutPtr<u8>>,
}
unsafe impl SafeRead for hostent {}

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
#[allow(non_camel_case_types)]
pub struct addrinfo {
    ai_flags: i32,
    ai_family: i32,
    ai_socktype: i32,
    ai_protocol: i32,
    ai_addrlen: socklen_t,
    ai_canonname: MutPtr<u8>,
    ai_addr: MutPtr<sockaddr>,
    ai_next: MutPtr<addrinfo>,
}
unsafe impl SafeRead for addrinfo {}

fn getaddrinfo(
    env: &mut Environment,
    node_name: MutPtr<u8>,
    serv_name: MutPtr<u8>,
    hints: ConstPtr<addrinfo>,
    res: MutPtr<MutPtr<addrinfo>>,
) -> i32 {
    if !env.options.network_access {
        log_dbg!(
            "Network access is disabled, getaddrinfo({:?}, {:?}, {:?}, {:?}) -> EAI_FAIL",
            node_name,
            serv_name,
            hints,
            res
        );
        return EAI_FAIL;
    }
    assert!(node_name.is_null()); // TODO
    let hint = env.mem.read(hints);
    let ai_flags = hint.ai_flags;
    assert_eq!(ai_flags, AI_PASSIVE);
    let ai_family = hint.ai_family;
    assert_eq!(ai_family, AF_INET);
    assert!(hint.ai_socktype == SOCK_STREAM || hint.ai_socktype == SOCK_DGRAM);
    assert!(
        hint.ai_protocol == IPPROTO_TCP || hint.ai_protocol == IPPROTO_UDP || hint.ai_protocol == 0
    );
    let ai_addrlen = hint.ai_addrlen;
    assert_eq!(ai_addrlen, 0);
    assert!(hint.ai_canonname.is_null());
    assert!(hint.ai_addr.is_null());
    assert!(hint.ai_next.is_null());
    let mut addr_info = hint;
    let port: u16 = env.mem.cstr_at_utf8(serv_name).unwrap().parse().unwrap();
    log_dbg!("getaddrinfo: port {}", port);
    let addr = sockaddr::from_ipv4_parts([0; 4], port);
    let tmp_addr = env.mem.alloc_and_write(addr);
    addr_info.ai_addr = tmp_addr;
    addr_info.ai_addrlen = guest_size_of::<sockaddr>();
    let tmp_addr_info = env.mem.alloc_and_write(addr_info);
    env.mem.write(res, tmp_addr_info);
    0 // Success
}

fn freeaddrinfo(env: &mut Environment, addrinfo: MutPtr<addrinfo>) {
    let addrinfo_val = env.mem.read(addrinfo);
    assert!(addrinfo_val.ai_next.is_null()); // TODO
    let ai_addrlen = addrinfo_val.ai_addrlen;
    assert_eq!(ai_addrlen, guest_size_of::<sockaddr>());
    env.mem.free(addrinfo_val.ai_addr.cast());
    env.mem.free(addrinfo.cast());
}

fn gethostbyname(env: &mut Environment, name: ConstPtr<u8>) -> MutPtr<hostent> {
    let name_str = env.mem.cstr_at_utf8(name).unwrap().to_string();
    log!("gethostbyname(\"{}\") => fake 127.0.0.1", name_str);

    // Allocate a fake IP address: 127.0.0.1
    let ip_size: GuestUSize = 4;
    let ip_ptr: MutPtr<u8> = env.mem.alloc(ip_size).cast();
    let ip_bytes: [u8; 4] = [127, 0, 0, 1];
    env.mem.bytes_at_mut(ip_ptr, ip_size).copy_from_slice(&ip_bytes);

    // Allocate addr_list: [ip_ptr, null_ptr] — two pointer slots
    let ptr_size = guest_size_of::<MutPtr<u8>>();
    let addr_list: MutPtr<MutPtr<u8>> = env.mem.alloc(ptr_size * 2).cast();
    env.mem.write(addr_list, ip_ptr);
    // Write null terminator after first entry
    let null_ptr: MutPtr<u8> = Ptr::null();
    env.mem.write(
        MutPtr::from_bits(addr_list.to_bits() + ptr_size),
        null_ptr,
    );

    // Allocate null alias list: [null]
    let aliases: MutPtr<MutPtr<u8>> = env.mem.alloc(ptr_size).cast();
    env.mem.write(aliases, null_ptr);

    // Allocate the hostent struct
    let h = hostent {
        h_name: ip_ptr,
        h_aliases: aliases,
        h_addrtype: AF_INET as i32,
        h_length: 4,
        h_addr_list: addr_list,
    };
	
    env.mem.alloc_and_write(h).cast()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(getaddrinfo(_, _, _, _)),
    export_c_func!(freeaddrinfo(_)),
    export_c_func!(gethostbyname(_)),
];