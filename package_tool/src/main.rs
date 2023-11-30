use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{
    ffi::OsStr,
    fs::File,
    io::{BufReader, Cursor, Read, Write},
    path::{Path, PathBuf},
    process::Command,
};
use structopt::{clap::arg_enum, StructOpt};
use url::Url;
use zip::{write::FileOptions, ZipArchive};

arg_enum! {
    enum ReleaseTarget {
        Dev,
        Rel
    }
}

arg_enum! {
    enum BuildTarget {
        Debug,
        Release
    }
}

#[derive(StructOpt)]
#[structopt()]
struct Opt {
    #[structopt(long, default_value = "")]
    target_platform: String,

    #[structopt(long)]
    release_target: ReleaseTarget,

    #[structopt(long)]
    build_target: BuildTarget,

    #[structopt(long)]
    aws_profile: String,

    #[structopt(long)]
    aws_path: Url,
}

#[derive(Deserialize)]
struct Version {
    pub version: String,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let target_path = if opt.target_platform.is_empty() {
        "target".to_owned()
    } else {
        format!("target.{}", opt.target_platform)
    };
    let mut path = PathBuf::from(target_path);
    match opt.build_target {
        BuildTarget::Debug => path.push("debug"),
        BuildTarget::Release => path.push("release"),
    }

    // Read the version file
    let (version_path, version) = get_version(&path).with_context(|| "Failed to get version")?;
    println!("Build Target: {}", opt.build_target);
    println!("Release Target: {}", opt.release_target);
    println!("Target Platform: {}", opt.target_platform);
    println!("Version: {}", version.version);

    let asma_zip_path = zip_asma(&path).with_context(|| "Failed to zip asma")?;

    println!("ZipFile written to {}", asma_zip_path.display());

    upload_to_s3(
        opt.release_target,
        opt.target_platform,
        version,
        &opt.aws_path,
        &opt.aws_profile,
        &version_path,
        &asma_zip_path,
    )
    .with_context(|| "Failed to upload to S3")?;
    Ok(())
}

fn upload_to_s3(
    target: ReleaseTarget,
    target_platform: String,
    version: Version,
    aws_path: &Url,
    aws_profile: &str,
    version_path: &PathBuf,
    asma_zip_path: &PathBuf,
) -> Result<()> {
    let target_platform = if target_platform.is_empty() {
        target_platform
    } else {
        format!(".{}", target_platform)
    };

    let asma_zip_url = aws_path
        .join(&format!(
            "latest-{}{}.zip",
            target.to_string().to_ascii_lowercase(),
            target_platform
        ))
        .expect("Failed to create asma_zip_url");

    let asma_versioned_zip_url = aws_path
        .join(&format!(
            "{}-{}{}.zip",
            version.version,
            target.to_string().to_ascii_lowercase(),
            target_platform
        ))
        .expect("Failed to create asma_zip_url");

    execute_command(
        "aws",
        [
            "s3",
            "cp",
            asma_zip_path
                .as_path()
                .to_str()
                .expect("Failed to stringify asma_zip_path"),
            &asma_zip_url.to_string(),
            "--profile",
            aws_profile,
        ],
    )
    .expect("Failed to upload asma to S3");

    execute_command(
        "aws",
        [
            "s3",
            "cp",
            &asma_zip_url.to_string(),
            &asma_versioned_zip_url.to_string(),
            "--profile",
            aws_profile,
        ],
    )
    .expect("Failed to upload asma to S3");

    let version_json_url = aws_path
        .join(&format!(
            "latest-{}{}.json",
            target.to_string().to_ascii_lowercase(),
            target_platform
        ))
        .expect("Failed to create version url");

    execute_command(
        "aws",
        [
            "s3",
            "cp",
            version_path
                .as_path()
                .to_str()
                .expect("Failed to stringify version_path"),
            &version_json_url.to_string(),
            "--profile",
            aws_profile,
        ],
    )
    .expect("Failed to upload version to S3");

    Ok(())
}

fn execute_command<I: IntoIterator<Item = S>, S: AsRef<OsStr>>(
    command: &str,
    args: I,
) -> Result<()> {
    let mut command = Command::new(command);
    command.args(args);
    let output = command.output().expect("Failed to execute aws");
    println!("status: {}", output.status);
    std::io::stdout().write_all(&output.stdout).unwrap();
    std::io::stderr().write_all(&output.stderr).unwrap();
    if let Some(code) = output.status.code() {
        if code == 0 {
            Ok(())
        } else {
            bail!("Process failed with exit code {}", code);
        }
    } else {
        bail!("Process did not return an exit code??");
    }
}

fn get_version(path: &PathBuf) -> Result<(PathBuf, Version)> {
    let version_file = Path::new(&path).join("version.json");
    let version = serde_json::from_reader::<_, Version>(BufReader::new(
        File::open(&version_file).with_context(|| "Failed to open version file")?,
    ))
    .with_context(|| "Failed to deserialize version")?;
    Ok((version_file, version))
}

fn zip_asma(path: &PathBuf) -> Result<PathBuf> {
    let asma_exe_path = Path::new(&path).join("asma.exe");
    let asma_zip_path = Path::new(&path).join("asma.zip");

    let mut asma_exe_bytes = Vec::new();
    let _ = File::open(asma_exe_path)
        .expect("Failed to open asma.exe")
        .read_to_end(&mut asma_exe_bytes)
        .expect("Failed to read asma.exe bytes");

    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    println!("Compressing...");
    // Write to a buffer
    let write_buf: Vec<u8> = Vec::new();
    let cursor = Cursor::new(write_buf);
    let mut zip_writer = zip::write::ZipWriter::new(cursor);
    zip_writer.start_file("asma.exe", options).unwrap();
    zip_writer.write_all(&asma_exe_bytes).unwrap();
    let cursor = zip_writer.finish().unwrap();
    let write_buf = cursor.into_inner();

    // Write to zip file prospectively
    println!("Writing...");
    std::fs::write(&asma_zip_path, &write_buf).unwrap();

    // Read back from the buffer to verify
    println!("Verifying...");
    let cursor = Cursor::new(std::fs::read(&asma_zip_path).unwrap());
    let mut zip_archive = match ZipArchive::new(cursor) {
        Ok(archive) => archive,
        Err(e) => bail!("Failed to open archive: {}", e.to_string()),
    };
    let mut asma_exe_result = zip_archive
        .by_name("asma.exe")
        .with_context(|| "Failed to find asma.exe in zip archive")?;
    let mut buf = Vec::new();
    asma_exe_result
        .read_to_end(&mut buf)
        .with_context(|| "Failed to read asma.exe")?;

    Ok(asma_zip_path)
}
