// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! A command line interface for the compiler driver
//!
//! The following exit status codes are used:
//! - `0` means compilation succeeded or no compilation was attempted
//! - `1` means compilation failed because of a source error
//! - `2` means compilation was misconfigured, or the command line arguments were malformed
//! - `3` means an internal compiler exception occurred

fn compile() -> denuocc::Result<bool> {
    let mut driver = denuocc::Driver::new();
    driver.parse_cli_args_from_env()?;
    driver.run()?;
    driver.report_messages();

    let success = driver.success();
    if success {
        driver.write_output()?;
    }
    Ok(success)
}

fn run() -> bool {
    env_logger::init();
    compile().unwrap_or_else(|e| e.print_and_exit())
}

fn ice_hook(p: &std::panic::PanicInfo) {
    eprintln!("error: an internal compiler exception has occurred");

    let message = match p.payload() {
        x if x.is::<String>() => x.downcast_ref::<String>().unwrap().as_str(),
        x if x.is::<&str>() => x.downcast_ref::<&str>().unwrap(),
        _ => "exception type could not be determined",
    };
    eprintln!("error: {}", message);
    eprintln!("");
    eprintln!("{:?}", backtrace::Backtrace::new());
    eprintln!("");
    eprintln!("please file a bug report");

    // Allow unwinding to occur (rather than exiting here) so that in the future we clean
    // up any kind of file system state, perhaps some cache mechanism
}

fn main() {
    std::panic::set_hook(Box::new(ice_hook));
    let result = std::panic::catch_unwind(run);
    match result {
        Ok(success) => std::process::exit(if success { 0 } else { 1 }),
        Err(_) => std::process::exit(3),
    }
}
