/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
// Allow the crate to have a non-snake-case name (touchHLE).
// This also allows items in the crate to have non-snake-case names.
#![allow(non_snake_case)]
fn main() -> Result<(), String> {
    // Use a larger stack size to prevent stack overflows from deeply nested
    // game callbacks (e.g. Zenonia 3's animation timer chain).
    let args: Vec<String> = std::env::args().collect();
    let builder = std::thread::Builder::new().stack_size(256 * 1024 * 1024);
    let handler = builder.spawn(move || {
        touchHLE::main(args.into_iter())
    }).unwrap();
    handler.join().unwrap()
}