use cc;
use std::env;
use std::collections::HashSet;

fn find_files(ptns: &[&str]) -> Vec<String> {
	let mut rt = HashSet::new();
	for ptn in ptns {
		if ptn.starts_with("-") {
			let ptn = &ptn[1..];
			for f in glob::glob(ptn).unwrap().filter_map(Result::ok) {
				rt.remove(&f);
			}
		} else {
			for f in glob::glob(ptn).unwrap().filter_map(Result::ok) {
				rt.insert(f);
			}
		}
	}
	rt.iter().map(|x| x.display().to_string()).collect()
}

#[cfg(target_os = "windows")]
fn init_builder() -> cc::Build {
	env::set_var("VSLANG", "1033");
	let mut cxxb = cc::Build::new();
	cxxb.cpp(true).std("c++20").flag("/EHsc").flag("/utf-8")
		.flag("/D_CRT_SECURE_NO_WARNINGS")
		.flag("/D_CRT_NONSTDC_NO_WARNINGS")
		.flag("/DUNICODE").flag("/D_UNICODE");
	cxxb
}
#[cfg(not(target_os = "windows"))]
fn init_builder() -> cc::Build {
	let mut cxxb = cc::Build::new();
	cxxb.cpp(true).std("c++20").flag("-Wall").flag("-Wextra")
		.flag("-Wno-unused-parameter")
		.flag("-Wno-unused-result")
		.flag("-Wno-multichar")
		.flag("-Wno-missing-field-initializers")
		.flag("-Wno-unknown-pragmas")
		.flag("-g");
	cxxb
}

#[allow(dead_code)]
pub fn build<T>(projname: &str, headers: &[&str], sources: &[&str], modify: T)
	where T: FnOnce(&mut cc::Build)
{
	let srcfiles = find_files(sources);
	for entry in &srcfiles {
		println!("cargo:rerun-if-changed={}", entry);
	}
	for entry in find_files(headers) {
		println!("cargo:rerun-if-changed={}", entry);
	}

	let from_vs = env::var("VisualStudioDir").map(|x| !x.is_empty()).unwrap_or(false);
	let is_debug = env::var("PROFILE").map(|x| x == "debug").unwrap_or(false);
	if from_vs {
		if is_debug {
			println!("cargo:rustc-link-arg-bins=/WHOLEARCHIVE:x64/Debug/{}.lib", projname);
			println!("cargo:rerun-if-changed=x64/Debug/{}.lib", projname);
		} else {
			println!("cargo:rustc-link-arg-bins=/WHOLEARCHIVE:x64/Release/{}.lib", projname);
			println!("cargo:rerun-if-changed=x64/Release/{}.lib", projname);
		}
	} else {
		let mut cxxb = init_builder();
		cxxb.files(srcfiles);
		modify(&mut cxxb);
		cxxb.compile(&format!("{}1", projname));
	}
}
