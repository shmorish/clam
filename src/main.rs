use gnu_readline_sys::readline;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

fn main() {
    let ps1 = "$ ";

    loop {
        let input = read_line(ps1);

        match input {
            Some(line) => {
                if line.is_empty() {
                    continue;
                }
                println!("Input: {}", line);
            }
            None => {
                // Ctrl+D (EOF) でループを抜ける
                println!();
                break;
            }
        }
    }
}

/// readlineを使って1行読み込む
/// メモリ安全性を確保するため、RAIIパターンでメモリ管理を行う
fn read_line(prompt: &str) -> Option<String> {
    // promptをnull終端のCStringに変換
    let c_prompt = CString::new(prompt).ok()?;

    unsafe {
        let char_ptr: *mut c_char = readline(c_prompt.as_ptr());

        if char_ptr.is_null() {
            return None;
        }

        // スコープを抜ける際に確実にfreeするためのRAIIガード
        let _guard = FreeGuard(char_ptr);

        let c_str = CStr::from_ptr(char_ptr);
        c_str.to_str().ok().map(|s| s.to_string())
    }
}

/// RAIIパターンでreadlineが返したポインタを確実にfreeする
struct FreeGuard(*mut c_char);

impl Drop for FreeGuard {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.0 as *mut libc::c_void);
        }
    }
}
