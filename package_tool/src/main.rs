use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{
    ffi::OsStr,
    fs::File,
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    process::Command,
};
use structopt::{clap::arg_enum, StructOpt};
use url::Url;
use zip::write::FileOptions;

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
    let mut path = PathBuf::from("target");
    match opt.build_target {
        BuildTarget::Debug => path.push("debug"),
        BuildTarget::Release => path.push("release"),
    }

    // Read the version file
    let (version_path, version) = get_version(&path).with_context(|| "Failed to get version")?;
    println!("Build Target: {}", opt.build_target);
    println!("Release Target: {}", opt.release_target);
    println!("Version: {}", version.version);

    let asma_zip_path = zip_asma(&path).with_context(|| "Failed to zip asma")?;

    println!("ZipFile written to {}", asma_zip_path.display());

    upload_to_s3(
        opt.release_target,
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
    version: Version,
    aws_path: &Url,
    aws_profile: &str,
    version_path: &PathBuf,
    asma_zip_path: &PathBuf,
) -> Result<()> {
    let asma_zip_url = aws_path
        .join(&format!(
            "latest-{}.zip",
            target.to_string().to_ascii_lowercase()
        ))
        .expect("Failed to create asma_zip_url");

    let asma_versioned_zip_url = aws_path
        .join(&format!(
            "{}-{}.zip",
            version.version,
            target.to_string().to_ascii_lowercase()
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
            "latest-{}.json",
            target.to_string().to_ascii_lowercase()
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

    let mut zip_writer = zip::write::ZipWriter::new(
        std::fs::File::create(&asma_zip_path).expect("Failed to create asma.zip"),
    );
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);
    zip_writer
        .start_file("asma.exe", options)
        .unwrap();
    zip_writer.write_all(&asma_exe_bytes).unwrap();
    zip_writer.finish().unwrap();

    Ok(asma_zip_path)
}
