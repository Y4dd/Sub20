//! Sub20's planner for the browser. Sub20 Lite plans with the same rule and
//! the same tests as the desktop app by calling these functions in
//! WebAssembly (`src/lib/web/planner.ts` loads them).
//!
//! The interface is plain exported functions over linear memory, with JSON
//! in and out, so no binding generator is needed: the page writes the
//! request into memory it got from `alloc`, calls `plan`, and reads the
//! answer from the packed pointer and length it returns.

use serde::Deserialize;
use sub20_core::job::{EncodeOptions, Trim};
use sub20_core::plan::{self, Plan};
use sub20_core::MediaInfo;

#[derive(Deserialize)]
struct Request {
    source: MediaInfo,
    trim: Option<Trim>,
    options: EncodeOptions,
    /// The largest short side the browser's encoder takes at this frame rate.
    #[serde(rename = "mostShortSide", default)]
    most_short_side: Option<u32>,
}

/// `{"plan": …}`, or `{"error": "…"}` with the planner's advice.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum Answer {
    Plan(Plan),
    Error(String),
}

/// Plans one clip from a JSON request (`source`, `trim`, `options`).
pub fn plan_json(request: &str) -> String {
    let answer = match serde_json::from_str::<Request>(request) {
        Ok(r) => match plan::plan_within(&r.source, r.trim, &r.options, r.most_short_side) {
            Ok(plan) => Answer::Plan(plan),
            Err(e) => Answer::Error(e.to_string()),
        },
        Err(e) => Answer::Error(format!("The planner couldn't read the request ({e}).")),
    };
    serde_json::to_string(&answer).unwrap_or_else(|_| r#"{"error":"The plan couldn't be written."}"#.into())
}

/// Memory for the page to write a request into.
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// Gives back memory from `alloc`, or an answer from `plan`.
///
/// # Safety
/// `ptr` and `len` must come from `alloc(len)` or from `plan`'s answer, once.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len));
}

/// Plans from the UTF-8 JSON request at `ptr`, `len` bytes long (which it
/// frees), and answers with the answer's pointer in the high 32 bits and its
/// length in the low 32.
///
/// # Safety
/// `ptr` and `len` must come from `alloc(len)`, filled with the request.
#[no_mangle]
pub unsafe extern "C" fn plan(ptr: *mut u8, len: usize) -> u64 {
    let bytes = Vec::from_raw_parts(ptr, len, len);
    let answer = plan_json(&String::from_utf8_lossy(&bytes)).into_bytes().into_boxed_slice();
    let (out, out_len) = (answer.as_ptr() as u64, answer.len() as u64);
    std::mem::forget(answer);
    (out << 32) | out_len
}

/// The next video bitrate after a result landed over its target.
#[no_mangle]
pub extern "C" fn lower_bitrate(video_kbps: u32, size: f64, target: f64, duration: f64) -> u32 {
    plan::lower_bitrate(video_kbps, size as u64, target as u64, duration)
}

/// How many times a too-large result is encoded again.
#[no_mangle]
pub extern "C" fn size_retries() -> u32 {
    plan::SIZE_RETRIES
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"{"duration":60,"sizeBytes":200000000,"hasVideo":true,"hasAudio":true,"width":1920,"height":1080,
        "displayWidth":1920,"displayHeight":1080,"rotation":0,"fps":60,"videoCodec":"h264","pixelFormat":"yuv420p",
        "videoKbps":26000,"audioCodec":"aac","audioKbps":160,"audioChannels":2,"audioTracks":1,"hdr":null}"#;

    #[test]
    fn plans_from_json_the_way_the_window_sends_it() {
        let options = serde_json::to_string(&EncodeOptions::default()).unwrap();
        let answer = plan_json(&format!(r#"{{"source":{SOURCE},"trim":{{"start":10,"end":40}},"options":{options}}}"#));
        let value: serde_json::Value = serde_json::from_str(&answer).unwrap();
        let plan = &value["plan"];
        assert_eq!(plan["duration"], 30.0, "{answer}");
        assert!(plan["videoKbps"].as_u64().unwrap() > 1000, "{answer}");
        assert_eq!(plan["trimmed"], true);
    }

    #[test]
    fn a_plan_keeps_within_the_encoder() {
        let options = serde_json::to_string(&EncodeOptions { target_mb: 500.0, ..EncodeOptions::default() }).unwrap();
        let answer =
            plan_json(&format!(r#"{{"source":{SOURCE},"trim":null,"options":{options},"mostShortSide":720}}"#));
        let value: serde_json::Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(value["plan"]["height"], 720, "{answer}");
    }

    #[test]
    fn a_plan_that_cannot_fit_says_why() {
        let options = serde_json::to_string(&EncodeOptions { target_mb: 1.0, ..EncodeOptions::default() }).unwrap();
        let long = SOURCE.replace(r#""duration":60"#, r#""duration":7200"#);
        let answer = plan_json(&format!(r#"{{"source":{long},"trim":null,"options":{options}}}"#));
        assert!(answer.contains("too small"), "{answer}");
        assert!(plan_json("not json").contains("couldn't read the request"));
    }
}
