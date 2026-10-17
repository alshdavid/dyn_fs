use std::fs;

// use dyn_fs::mem::MemoryFileSystem;
use dyn_fs::FileSystem;
use dyn_fs::os::OsFileSystem;

fn main() -> std::io::Result<()> {
  let os_fs = OsFileSystem::default();
  // TODO
  // let mem_fs = MemoryFileSystem::default();

  use_dyn_fs(&os_fs)?;
  // TODO
  // use_dyn_fs(&mem_fs)?;

  Ok(())
}

fn use_dyn_fs(fs: &dyn FileSystem) -> std::io::Result<()> {
  let exe_path = std::env::current_exe()?;
  let path = exe_path.parent().unwrap();
  let file = path.join("basic_sync.txt");
  println!("Dir Target:   {:?}", path);
  println!("Dir Exists:   {:?}", fs::exists(&path)?);
  println!("File Target:  {:?}", file);
  println!("File Exists:  {:?}", fs::exists(&file)?);

  fs.write(&file, b"hello world")?;

  let contents = fs.read_to_string(&file)?;
  println!("Contents:     {}", contents);

  fs.remove_file(&file)?;
  Ok(())
}
