use std::time::Instant;

lazy_static! {
    // Record a starting point when the program begins.
    static ref START: Instant = Instant::now();
}

#[no_mangle]
pub extern "C" fn osdp_millis_now() -> i64 {
    let elapsed = START.elapsed();
    elapsed.as_millis() as i64
}
