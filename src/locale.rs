use nix::libc::{self, CODESET, LC_ALL};
use std::env;
use std::ffi::CStr;

pub fn check_utf8_locale() -> anyhow::Result<()> {
    initialize_from_env();

    let encoding = get_encoding();

    if ["US-ASCII", "UTF-8"].contains(&encoding.as_str()) {
        Ok(())
    } else {
        let env = env::var("LC_ALL")
            .map(|v| format!("LC_ALL={}", v))
            .or(env::var("LC_CTYPE").map(|v| format!("LC_CTYPE={}", v)))
            .or(env::var("LANG").map(|v| format!("LANG={}", v)))
            .unwrap_or("".to_string());

        Err(anyhow::anyhow!("asciinema requires ASCII or UTF-8 character encoding. The environment ({}) specifies the character set \"{}\". Check the output of `locale` command.", env, encoding))
    }
}

pub fn initialize_from_env() {
    unsafe {
        libc::setlocale(LC_ALL, b"\0".as_ptr() as *const libc::c_char);
    };
}

fn get_encoding() -> String {
    let codeset = unsafe { CStr::from_ptr(libc::nl_langinfo(CODESET)) };

    let mut encoding = codeset
        .to_str()
        .expect("Locale codeset name is not a valid UTF-8 string")
        .to_owned();

    if encoding == "ANSI_X3.4-1968" {
        encoding = "US-ASCII".to_owned();
    }

    encoding
}
