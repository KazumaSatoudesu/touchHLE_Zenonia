/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! AVAudioSession (stub)
//!
//! Zenonia 3 calls [AVAudioSession sharedInstance] and then sets category/active.
//! We provide a minimal singleton stub that silently accepts all common calls.

use crate::objc::{id, msg, msg_class, nil, ClassExports, HostObject, NSZonePtr};
use crate::objc_classes;
use crate::dyld::{ConstantExports, HostConstant};

struct AVAudioSessionHostObject;
impl HostObject for AVAudioSessionHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation AVAudioSession: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(AVAudioSessionHostObject);
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)sharedInstance {
    // Return a lazily-created singleton stored in a static.
    // For simplicity we just alloc a new one each time if needed;
    // the game only calls this once so it won't matter.
    log!("AVAudioSession sharedInstance called (stub)");
    let instance: id = msg_class![env; AVAudioSession alloc];
    let instance: id = msg![env; instance init];
    instance
}

- (id)init {
    this
}

// setCategory:error: — ignore, return YES (success)
- (bool)setCategory:(id)_category error:(id)_error {
    log!("AVAudioSession setCategory:error: (stub, ignoring)");
    true
}

// setCategory:withOptions:error: — ignore, return YES
- (bool)setCategory:(id)_category withOptions:(u32)_options error:(id)_error {
    log!("AVAudioSession setCategory:withOptions:error: (stub, ignoring)");
    true
}

// setActive:error: — ignore, return YES
- (bool)setActive:(bool)_active error:(id)_error {
    log!("AVAudioSession setActive:error: (stub, ignoring)");
    true
}

// setActive:withOptions:error: — ignore, return YES
- (bool)setActive:(bool)_active withOptions:(u32)_options error:(id)_error {
    log!("AVAudioSession setActive:withOptions:error: (stub, ignoring)");
    true
}

// category — return nil (acceptable for a stub)
- (id)category {
    log!("AVAudioSession category (stub, returning nil)");
    nil
}

// delegate — return nil
- (id)delegate {
    nil
}

// setDelegate: — ignore
- (())setDelegate:(id)_delegate {
    log!("AVAudioSession setDelegate: (stub, ignoring)");
}

// otherAudioPlaying — return NO
- (bool)isOtherAudioPlaying {
    false
}

@end

};

pub const CONSTANTS: ConstantExports = &[
    ("_AVAudioSessionCategoryAmbient",     HostConstant::NSString("AVAudioSessionCategoryAmbient")),
    ("_AVAudioSessionCategoryPlayback",    HostConstant::NSString("AVAudioSessionCategoryPlayback")),
    ("_AVAudioSessionCategorySoloAmbient", HostConstant::NSString("AVAudioSessionCategorySoloAmbient")),
    ("_AVAudioSessionCategoryRecord",      HostConstant::NSString("AVAudioSessionCategoryRecord")),
];