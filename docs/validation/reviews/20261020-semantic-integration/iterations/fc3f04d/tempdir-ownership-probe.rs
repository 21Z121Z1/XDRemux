use std::{fs,io,path::PathBuf};
fn main() {
 let fixed=std::env::args().nth(1).as_deref()==Some("fixed");
 let outer=tempfile::tempdir().unwrap();
 let mut owners=Vec::new();
 let paths:Vec<PathBuf>=(0..2).map(|_| {
  if fixed {
   let owner=tempfile::Builder::new().prefix("xdremux-cli-batch-").tempdir_in(outer.path()).unwrap();
   let path=owner.path().to_owned();owners.push(owner);path
  } else {
   // Equal SystemTime values are allowed; simulate a coarse clock deterministically.
   // This is the exact legacy PID+stamp naming and create_dir_all behavior.
   let stamp=42_u128;
   let path=outer.path().join(format!("xdremux-cli-batch-{}-{stamp}",std::process::id()));
   fs::create_dir_all(&path).unwrap();path
  }
 }).collect();
 let input=outer.path().join("immutable-input");fs::write(&input,b"original media sentinel").unwrap();
 fs::write(paths[1].join("second-owner-sentinel"),b"second owner").unwrap();
 println!("mode={} first={} second={} same_path={}",if fixed {"fixed"} else {"legacy-coarse-clock"},paths[0].display(),paths[1].display(),paths[0]==paths[1]);
 if fixed {drop(owners.remove(0));} else {fs::remove_dir_all(&paths[0]).unwrap();}
 let attempt=fs::OpenOptions::new().write(true).create_new(true).open(paths[1].join(".capture.unique.tmp.heic"));
 println!("after first cleanup: second_parent_exists={} second_sentinel_exists={} create_error={:?}",paths[1].is_dir(),paths[1].join("second-owner-sentinel").is_file(),attempt.as_ref().err().map(|e|(e.kind(),e.raw_os_error())));
 assert_eq!(fs::read(input).unwrap(),b"original media sentinel");
 if !fixed {assert_eq!(attempt.as_ref().unwrap_err().kind(),io::ErrorKind::NotFound);}
 assert_ne!(paths[0],paths[1],"independent tests must never share their directory lease");
 assert!(attempt.is_ok(),"one test cleanup must not remove another test's publication parent");
}
