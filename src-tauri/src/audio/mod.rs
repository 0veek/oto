pub mod capture;
pub mod wav;

// Re-exported for pipeline (Task 9+); may appear unused until wired.
#[allow(unused_imports)]
pub use capture::AudioRecorder;
