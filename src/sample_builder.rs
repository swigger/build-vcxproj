use cc;
use std::env;
use std::collections::HashSet;
use bitflags::bitflags;

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
fn init_builder(is_debug: bool) -> cc::Build {
	env::set_var("VSLANG", "1033");
	let mut cxxb = cc::Build::new();
	cxxb.cpp(true).std("c++20").flag("/EHsc").flag("/utf-8")
		.flag("/D_CRT_SECURE_NO_WARNINGS")
		.flag("/D_CRT_NONSTDC_NO_WARNINGS")
		.flag("/DUNICODE").flag("/D_UNICODE").flag("/Zi").flag("/FS");
	if is_debug {
		cxxb.flag("/Od").flag("/RTC1").flag("/D_DEBUG");
	} else {
		cxxb.flag("/O2").flag("/DNDEBUG");
	}
	cxxb
}
#[cfg(not(target_os = "windows"))]
fn init_builder(isdebug: bool) -> cc::Build {
	let mut cxxb = cc::Build::new();
	cxxb.cpp(true).std("c++20").flag("-Wall").flag("-Wextra")
		.flag("-Wno-unused-parameter")
		.flag("-Wno-unused-result")
		.flag("-Wno-multichar")
		.flag("-Wno-missing-field-initializers")
		.flag("-Wno-unknown-pragmas")
		.flag("-g");
	if isdebug {
		cxxb.flag("-O0");
	} else {
		cxxb.flag("-O2");
	}
	cxxb
}

bitflags! {
	pub struct BuildOptions: u32 {
		const BuildWithVCLib = 0b01;
		const Dummy = 0b10;
	}
}


#[allow(dead_code)]
pub fn build<T>(projname: &str, headers: &[&str], sources: &[&str], incdirs:&[&str],
                opt:BuildOptions, modify: T)
	where T: FnOnce(&mut cc::Build)
{
	let srcfiles = find_files(sources);
	for entry in &srcfiles {
		println!("cargo:rerun-if-changed={}", entry);
	}
	for entry in find_files(headers) {
		println!("cargo:rerun-if-changed={}", entry);
	}

	let from_vs = if opt.contains(BuildOptions::BuildWithVCLib) {
		env::var("VisualStudioDir").map(|x| !x.is_empty()).unwrap_or(false)
	} else {
		false
	};
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
		let mut new_incdirs = Vec::new();
		for dir in incdirs {
			let expanded_dir = if dir.starts_with("~") {
				if let Ok(home) = env::var("HOME") {
					dir.replacen("~", &home, 1)
				} else {
					dir.to_string()
				}
			} else {
				dir.to_string()
			};
			
			if std::path::Path::new(&expanded_dir).exists() {
				new_incdirs.push(expanded_dir);
			}
		}
		crate::vcpkg::add_inc_paths(&mut new_incdirs);
		let mut lib_dirs = Vec::new();
		crate::vcpkg::add_lib_paths(is_debug, &mut lib_dirs);
		for libdir in lib_dirs {
			println!("cargo:rustc-link-search={}", libdir);
		}
		let mut cxxb = init_builder(is_debug);
		cxxb.files(srcfiles);
		cxxb.includes(&new_incdirs);
		modify(&mut cxxb);
		cxxb.compile(&format!("{}", projname));
	}
}
