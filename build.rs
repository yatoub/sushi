use std::{fs, io::Write, path::PathBuf};

use clap::CommandFactory;
use clap_mangen::Man;

// Reproduit la définition de Cli pour build.rs (ne peut pas importer lib.rs).
// Doit rester synchronisé avec src/lib.rs.
#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: Option<String>,
    #[arg(long, value_name = "[USER@]HOST[:PORT]", conflicts_with_all = ["jump", "wallix"])]
    direct: Option<String>,
    #[arg(long, value_name = "[USER@]HOST[:PORT]", conflicts_with_all = ["direct", "wallix"])]
    jump: Option<String>,
    #[arg(long, value_name = "[USER@]HOST[:PORT]", conflicts_with_all = ["direct", "jump"])]
    wallix: Option<String>,
    #[arg(short, long, value_name = "USER")]
    user: Option<String>,
    #[arg(short, long, value_name = "PORT")]
    port: Option<u16>,
    #[arg(short, long, value_name = "PATH")]
    key: Option<String>,
    #[arg(short, long)]
    verbose: bool,
    #[arg(long)]
    validate: bool,
    #[arg(long, conflicts_with_all = ["validate", "direct", "jump", "wallix"])]
    import_ssh_config: bool,
    #[arg(long, value_name = "FILE", requires = "import_ssh_config")]
    ssh_config_path: Option<String>,
    #[arg(long, value_name = "FILE", requires = "import_ssh_config")]
    output: Option<String>,
    #[arg(long, requires = "import_ssh_config")]
    dry_run: bool,
    #[arg(long, value_name = "FORMAT", conflicts_with_all = ["validate", "direct", "jump", "wallix", "import_ssh_config"])]
    export: Option<String>,
    #[arg(long = "export-output", value_name = "FILE", requires = "export")]
    export_output: Option<String>,
    #[arg(long = "export-filter", value_name = "QUERY", requires = "export")]
    export_filter: Option<String>,
    #[arg(long, conflicts_with_all = ["validate", "direct", "jump", "wallix", "import_ssh_config", "export"])]
    list: bool,
    #[arg(long = "list-filter", value_name = "QUERY", requires = "list")]
    list_filter: Option<String>,
    #[arg(long, value_name = "GROUP", conflicts_with_all = ["validate", "direct", "jump", "wallix", "import_ssh_config", "export", "list"])]
    exec_group: Option<String>,
    #[arg(long = "exec-cmd", value_name = "CMD", requires = "exec_group")]
    exec_cmd: Option<String>,
    #[arg(
        long = "exec-timeout",
        value_name = "SECS",
        requires = "exec_group",
        default_value = "30"
    )]
    exec_timeout: u64,
}

fn main() {
    // Génère la manpage dans target/man/ pour cargo-deb et le spec RPM.
    let man_dir = PathBuf::from("target/man");
    fs::create_dir_all(&man_dir).expect("cannot create target/man");

    let cmd = Cli::command();
    let man = Man::new(cmd);

    let mut buf = Vec::new();
    man.render(&mut buf).expect("manpage render failed");

    // Compress avec gzip pour les paquets DEB/RPM.
    let gz_path = man_dir.join("susshi.1.gz");
    let file = fs::File::create(&gz_path).expect("cannot create susshi.1.gz");
    let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::best());
    encoder.write_all(&buf).expect("gzip write failed");
    encoder.finish().expect("gzip finish failed");

    // Version non-compressée pour le spec RPM (rpmbuild compresse lui-même).
    let raw_path = man_dir.join("susshi.1");
    fs::write(raw_path, &buf).expect("cannot write susshi.1");
}
