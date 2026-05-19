use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_dir = "proto";
    let out_dir = "src/rust_grpc";

    // 1.生成pb代码之前，先删除原来的rs文件
    let _ = fs::remove_dir_all(out_dir);
    fs::create_dir_all(out_dir)?;

    // 2.读取proto文件
    let mut proto_files = Vec::new();
    for entry in fs::read_dir(proto_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("proto") {
            proto_files.push(path.to_string_lossy().into_owned());
        }
    }
    proto_files.sort();

    // 3.生成pb代码，这里指定了pb代码输出目录位置
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(out_dir)
        .compile_protos(&proto_files, &[proto_dir.to_string()])?;

    // 生成mod.rs文件
    let mut mods = String::new();
    for proto in &proto_files {
        let name = Path::new(proto)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("invalid proto file name")?;
        mods.push_str(&format!("pub mod {};\n", name));
    }

    // 4.将模块列表写入mod.rs中
    fs::write(format!("{}/mod.rs", out_dir), mods)?;

    Ok(())
}
