use crate::error::RinfError;
use crate::shutdown::SHUTDOWN_EVENTS;
use allo_isolate::{IntoDart, Isolate, ZeroCopyBuffer};
use os_thread_local::ThreadLocal;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::thread;

static DART_ISOLATE: Mutex<Option<Isolate>> = Mutex::new(None);

#[no_mangle]
pub extern "C" fn prepare_isolate_extern(port: i64) {
    let dart_isolate = Isolate::new(port);
    let mut guard = match DART_ISOLATE.lock() {
        Ok(inner) => inner,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.replace(dart_isolate);
}

// We use `os_thread_local` so that when the program fails
// and the main thread exits unexpectedly,
// the whole Rust async runtime shuts down accordingly.
// Without this solution,
// zombie threads inside the Rust async runtime might outlive the app.
// This `ThreadLocal` is intended to be used only on the main thread,
type ShutdownDropperLock = OnceLock<ThreadLocal<ShutdownDropper>>;
static SHUTDOWN_DROPPER: ShutdownDropperLock = OnceLock::new();

/// Notifies Rust that Dart thread has exited when dropped.
pub struct ShutdownDropper;

impl Drop for ShutdownDropper {
    fn drop(&mut self) {
        SHUTDOWN_EVENTS.dart_stopped.set();
        SHUTDOWN_EVENTS.rust_stopped.wait();
    }
}

pub fn start_rust_logic_real<F, T>(main_fn: F) -> Result<(), RinfError>
where
    F: Fn() -> T + Send + 'static,
{
    // Enable console output for panics.
    #[cfg(debug_assertions)]
    {
        #[cfg(not(feature = "backtrace"))]
        {
            std::panic::set_hook(Box::new(|panic_info| {
                crate::debug_print!("A panic occurred in Rust.\n{panic_info}");
            }));
        }
        #[cfg(feature = "backtrace")]
        {
            std::panic::set_hook(Box::new(|panic_info| {
                let backtrace = backtrace::Backtrace::new();
                crate::debug_print!("A panic occurred in Rust.\n{panic_info}\n{backtrace:?}");
            }));
        }
    }

    // Prepare the shutdown dropper that will notify the Rust async runtime
    // after Dart thread has exited.
    // This code assumes that this is the main thread.
    let thread_local = ThreadLocal::new(|| ShutdownDropper);
    let _ = SHUTDOWN_DROPPER.set(thread_local);

    // Spawn the thread holding the async runtime.
    thread::spawn(move || {
        // In debug mode, shutdown events could have been set
        // after Dart's hot restart.
        #[cfg(debug_assertions)]
        {
            // Terminates the previous async runtime threads in Rust.
            SHUTDOWN_EVENTS.dart_stopped.set();
            // Clears the shutdown events as if the app has started fresh.
            SHUTDOWN_EVENTS.dart_stopped.clear();
            SHUTDOWN_EVENTS.rust_stopped.clear();
        }
        // Long-blocking function that runs throughout the app lifecycle.
        main_fn();
        // After the Rust async runtime is closed,
        // tell the main Dart thread to stop blocking before exit.
        SHUTDOWN_EVENTS.rust_stopped.set();
    });

    Ok(())
}

#[no_mangle]
pub extern "C" fn stop_rust_logic_extern() {
    SHUTDOWN_EVENTS.dart_stopped.set();
}

pub fn send_rust_signal_real(
    message_id: i32,
    message_bytes: Vec<u8>,
    binary: Vec<u8>,
) -> Result<(), RinfError> {
    // When `DART_ISOLATE` is not initialized, just return the error.
    // This can happen when running test code in Rust.
    let guard = match DART_ISOLATE.lock() {
        Ok(inner) => inner,
        Err(poisoned) => poisoned.into_inner(),
    };
    let dart_isolate = guard.as_ref().ok_or(RinfError::NoDartIsolate)?;

    // If a `Vec<u8>` is empty, we can't just simply send it to Dart
    // because panic can occur from null pointers.
    // Instead, we will reconstruct the empty vector from the Dart side.
    let message_filled = !message_bytes.is_empty();
    let binary_filled = !binary.is_empty();

    dart_isolate.post(
        vec![
            message_id.into_dart(),
            if message_filled {
                ZeroCopyBuffer(message_bytes).into_dart()
            } else {
                ().into_dart()
            },
            if binary_filled {
                ZeroCopyBuffer(binary).into_dart()
            } else {
                ().into_dart()
            },
        ]
        .into_dart(),
    );

    Ok(())
}
