pub mod vcpkg;
pub mod sample_builder;
use std::{fs, io, env};
use std::path::Path;
use std::process::{Command, ExitStatus};
use std::time::SystemTime;
use std::io::BufReader;
use xml::reader::{EventReader, XmlEvent};
use cc;

pub fn system(command_line: &str) -> io::Result<ExitStatus> {
    #[cfg(target_os = "windows")]
        let (shell, flag) = ("cmd", "/C");

    #[cfg(not(target_os = "windows"))]
        let (shell, flag) = ("sh", "-c");

    Command::new(shell)
        .arg(flag)
        .arg(command_line)
        .status()
}

pub fn need_build<T: AsRef<Path>, I: IntoIterator<Item = T>>(target: T, deps: I) -> bool {
    let target_path = target.as_ref();
    let target_metadata = fs::metadata(target_path);

    let target_mod_time = match target_metadata {
        Ok(metadata) => metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        Err(_) => return true, // Target does not exist, need to build
    };

    for dep in deps {
        let dep_path = dep.as_ref();
        match fs::metadata(dep_path) {
            Ok(metadata) => {
                let dep_mod_time = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                if dep_mod_time > target_mod_time {
                    return true; // Dependency is newer than target, need to build
                }
            },
            Err(_) => {
                eprintln!("Warning: Dependency file not found: {:?}", dep_path);
                // You might want to handle missing dependencies differently,
                // e.g., return `true` to indicate a build is needed due to missing input.
            },
        }
    }

    // All dependencies are older than target, no need to build
    false
}

#[derive(Debug, Default)]
pub struct Vcxproj {
    // the path to the library .vcxproj file, will read source files from this file
    pub lib_proj: String,
    pub condition: String,
    // on input, add some other include dirs. on output, the target include dirs
    pub include_dirs: Vec<String>,
    // on input, add some other lib dirs. on output, the target lib dirs
    pub lib_dirs: Vec<String>,
    pub sources: Vec<String>,
    pub flags: Vec<String>,
    pub target: String,  // target basename
    pub target_fn: String, // target full path
}

impl Vcxproj {
    pub fn new(proj_fn:&str, is_debug:bool) -> Self {
        Self {
            lib_proj: proj_fn.to_string(),
            condition: if is_debug {"Debug|x64"} else {"Release|x64"}.to_string(),
            ..Default::default()
        }
    }

    fn is_link(&self, v: &Vec<&str>) -> bool {
        let p = v.join("/");
        let p = Path::new(p.as_str());
        p.symlink_metadata().map(|x| x.file_type().is_symlink()).unwrap_or(false)
    }

    fn rela_path(&self, x: &str) -> Option<String> {
        if x.is_empty() {
            return None;
        }
        if ! Path::new(x).is_relative() {
            return None;
        }
        let mut items: Vec<&str> = self.lib_proj.split(|c| c=='/' || c=='\\').collect();
        if items.len() > 0 {
            items.pop();
        }
        items.extend(x.split(|c| c=='/' || c=='\\'));
        let mut vec2 = Vec::new();
        for item in items {
            match item {
                ""|"." => continue,
                ".." => {
                    if vec2.is_empty() || vec2[vec2.len()-1] == ".." || self.is_link(&vec2) {
                        vec2.push(item);
                    } else {
                        vec2.pop();
                    }
                }
                _ => vec2.push(item),
            };
        }
        if vec2.is_empty() {
            None
        } else {
            Some(vec2.join("/"))
        }
    }

    fn load_vcxproj(&mut self) -> Result<(), std::io::Error> {
        let file = fs::File::open(self.lib_proj.as_str())?;
        let file = BufReader::new(file);
        let parser = EventReader::new(file);
        let mut xml_paths = Vec::new();
        let mut skip = Vec::new();
        let mut cur_path = String::new();
        fn sum(iter: std::slice::Iter<i32>) -> i32 {
            let mut sum = 0;
            for i in iter {
                sum += i;
            }
            sum
        }

        for e in parser {
            match e {
                Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                    xml_paths.push(name.local_name.clone());
                    skip.push(0);
                    cur_path = xml_paths.join("/");
                    match cur_path.as_str() {
                        "Project/ItemGroup/ClCompile" => {
                            attributes.iter().find(|&x| x.name.local_name == "Include").map(|x| {
                                if let Some(p) = self.rela_path(&x.value) {
                                    self.sources.push(p);
                                }
                            });
                        }
                        "Project/ItemDefinitionGroup" => {
                            attributes.iter().find(|&x| x.name.local_name == "Condition").map(|x| {
                                if ! x.value.contains(self.condition.as_str()) {
                                    skip.pop();
                                    skip.push(1);
                                }
                            });
                        }
                        _ => {}
                    };
                }
                Ok(XmlEvent::EndElement{name:_}) => {
                    xml_paths.pop();
                    skip.pop();
                }
                Ok(XmlEvent::Characters(heh)) => {
                    if sum(skip.iter()) == 0 && cur_path.as_str() == "Project/ItemDefinitionGroup/ClCompile/AdditionalIncludeDirectories" {
                        heh.split(";").for_each(|x| {
                            if let Some(p) = self.rela_path(x) {
                                self.include_dirs.push(p);
                            }
                        });
                    }
                }
                Err(e) => {
                    return Err(io::Error::other(e));
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn basename(&self) -> String{
        Path::new(self.lib_proj.as_str()).file_stem()
            .map_or(None, |x| x.to_str())
            .map_or("".to_string(), |x| x.to_string())
    }

    pub fn find_lib(&self, name: &str) -> bool {
        let is_windows = cfg!(target_os = "windows");
        for p in &self.lib_dirs {
            let p1 = if is_windows {
                format!("{}/{}.lib", p, name)
            } else {
                format!("{}/lib{}.a", p, name)
            };
            if Path::new(p1.as_str()).is_file() {
                return true;
            }
        }
        false
    }

    pub fn load_config(&mut self) -> bool {
        self.include_dirs.retain(|x| Path::new(x).is_dir());
        self.lib_dirs.retain(|x| Path::new(x).is_dir());
        let q = self.load_vcxproj();
        let is_debug = self.condition.contains("Debug");
        vcpkg::add_lib_paths(is_debug, &mut self.lib_dirs);
        vcpkg::add_inc_paths(&mut self.include_dirs);
        self.target = self.basename();
        let is_windows = cfg!(target_os = "windows");
        let mut def_flags: Vec<_> = if is_windows {
            vec!["/EHsc", "/utf-8", "/D_CRT_SECURE_NO_WARNINGS", "/D_CRT_NONSTDC_NO_WARNINGS",
                 "/DUNICODE", "/D_UNICODE", "/Zi", "/FS", "/W3"]
        } else {
            vec!["-Wno-unused-parameter", "-Wno-unused-result", "-Wno-multichar",
                "-Wno-missing-field-initializers", "-g"]
        };
        if is_windows {
            let tgt_dir = format!("x64/{}", if is_debug {"Debug"} else {"Release"});
            let tgt_dir = self.rela_path(tgt_dir.as_str()).unwrap_or("".to_string());
            self.target_fn = format!("{}/{}.lib", tgt_dir, self.target);
            self.lib_dirs.push(tgt_dir);
            if is_debug {
                def_flags.push("/Od");
            } else {
                def_flags.push("/O2");
            }
        } else {
            if is_debug {
                def_flags.push("-O0");
            } else {
                def_flags.push("-O3");
            }
        }
        self.flags.extend(def_flags.iter().map(|x| x.to_string()));
        return q.is_ok();
    }
}

#[cfg(target_os = "windows")]
fn find_rc() -> Option<Command> {
    if let Some(tl) = cc::windows_registry::find_tool("x86_64", "cl.exe") {
        for (name, val) in tl.env() {
            let mut s1 = name.to_str()?.to_string();
            s1.make_ascii_lowercase();
            if s1 == "path" {
                if let Some(path) = val.to_str() {
                    for path1 in path.split(';') {
                        let rc_path = Path::new(path1).join("rc.exe");
                        if rc_path.exists() && rc_path.is_file() {
                            let mut command = Command::new(rc_path);
                            for (k,v) in tl.env() {
                                command.env(k, v);
                            }
                            return Some(command);
                        }
                    }
                }
                break;
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
pub fn compile_rc(src: &str) -> Option<()> {
    let mut cmd = find_rc()?;
    let outdir = env::var("OUT_DIR").ok()?;
    let outname = format!("{}\\{}.res", outdir, Path::new(src).file_stem()?.to_str()?);
    cmd.arg("/fo").arg(&outname).arg(src);
    let output = match cmd.output() {
        Ok(output) => output,
        Err(e) => panic!("Failed to run rc.exe: {}", e),
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stdout);
        panic!("rc.exe failed with status: {} {} bbb", output.status, stderr);
    }
    println!("cargo:rerun-if-changed={}", src);
    println!("cargo:rustc-link-arg-bins={}", outname);
    Some(())
}
#[cfg(not(target_os = "windows"))]
pub fn compile_rc(src: &str) -> Option<()> {
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut vcx = Vcxproj::new("../cpp_py/ctp_server/vs.proj/ctp_server_cpp.vcxproj", true);
        vcx.load_config();
        println!("{:?}", vcx);
    }
}
