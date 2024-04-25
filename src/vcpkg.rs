#[allow(unused_imports)]
use std::fs;
#[allow(unused_imports)]
use std::path::Path;

#[cfg(target_os = "windows")]
fn base_path() -> Option<String> {
	if let Ok(path) = std::env::var("LOCALAPPDATA") {
		let path = format!("{}/vcpkg/vcpkg.path.txt", path);
		if let Ok(content) = fs::read_to_string(path) {
			let path = content.trim();
			if Path::new(&path).is_dir() {
				return Some(path.to_string());
			}
		}
	}
	None
}

#[cfg(target_os = "windows")]
pub fn add_lib_paths(isdbg: bool, vs: &mut Vec<String>) {
	let dbg = if isdbg { "/debug" } else { "" };
	// get path from env LOCALAPPDATA\vcpkg\vcpkg.path.txt
	if let Some(path) = base_path() {
		let path1 = format!("{}/installed/x64-windows-static{}/lib", path, dbg);
		let path2 = format!("{}/installed/x64-windows{}/lib", path, dbg);
		// check if path1 is directory
		if Path::new(&path1).is_dir() {
			vs.push(path1);
		}
		if Path::new(&path2).is_dir() {
			vs.push(path2);
		}
	}
}

#[cfg(target_os = "windows")]
pub fn add_inc_paths(vs: &mut Vec<String>) {
	if let Some(path) = base_path() {
		let path1 = format!("{}/installed/x64-windows-static/include", path);
		let path2 = format!("{}/installed/x64-windows/include", path);
		if Path::new(&path1).is_dir() {
			vs.push(path1);
		}
		if Path::new(&path2).is_dir() {
			vs.push(path2);
		}
	}
}

#[cfg(not(target_os = "windows"))]
pub fn add_lib_paths(_isdbg: bool, _vs: &mut Vec<String>) {}
#[cfg(not(target_os = "windows"))]
pub fn add_inc_paths(_vs: &mut Vec<String>) {}
