//! Diagnostic rapide de connexion SSH.
//!
//! La fonction [`probe`] exécute un one-liner bash sur le serveur distant et
//! retourne les métriques système dans un [`ProbeResult`].

use crate::config::{ConnectionMode, ResolvedServer};
use crate::ssh::client::build_ssh_args;
use anyhow::Result;
use std::process::Command;

/// One-liner bash envoyé en argument SSH.
/// Retourne exactement 5 lignes :
/// 1. kernel (`uname -r`)
/// 2. modèle CPU (première entrée `/proc/cpuinfo`)
/// 3. load average 1/5/15m
/// 4. RAM : `<pct_used> <total_bytes>`
/// 5. Disque `/` : `<pct_used> <total_bytes>`
const PROBE_CMD: &str = concat!(
    "uname -r; ",
    "awk '/^model name/{sub(/.*: /,\"\"); print; exit}' /proc/cpuinfo; ",
    "uptime | awk -F'load average:' '{print $2}' | xargs; ",
    "free -b | awk '/^Mem/{printf \"%.0f %.0f\\n\", $3/$2*100, $2}'; ",
    "df -B1 / | awk 'NR==2{printf \"%.0f %.0f\\n\", $3/$2*100, $2}'",
);

// ─── Types publics ────────────────────────────────────────────────────────────

/// Métriques système collectées par [`probe`].
#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub kernel: String,
    pub cpu_model: String,
    /// Load average formaté : `"0.42, 0.38, 0.31"`
    pub load: String,
    /// Pourcentage de RAM utilisée (0–100).
    pub ram_pct: u8,
    /// RAM totale en Go.
    pub ram_total_gb: f32,
    /// Pourcentage de disque `/` utilisé (0–100).
    pub disk_pct: u8,
    /// Capacité disque `/` en Go.
    pub disk_total_gb: f32,
}

/// État courant d'un diagnostic lancé sur un serveur.
#[derive(Debug, Default, Clone)]
pub enum ProbeState {
    /// Aucun diagnostic en cours ou résultat effacé.
    #[default]
    Idle,
    /// Diagnostic en attente de réponse SSH.
    Running,
    /// Résultat disponible.
    Done(ProbeResult),
    /// Erreur (message lisible).
    Error(String),
}

// ─── Parsing ──────────────────────────────────────────────────────────────────

impl ProbeResult {
    /// Parse les 5 lignes retournées par `PROBE_CMD`.
    pub fn parse(raw: &str) -> Result<Self> {
        let lines: Vec<&str> = raw
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();

        if lines.len() < 5 {
            anyhow::bail!(
                "sortie inattendue ({} lignes au lieu de 5) :\n{}",
                lines.len(),
                raw
            );
        }

        let (ram_pct, ram_total_gb) = parse_pct_bytes(lines[3], "RAM")?;
        let (disk_pct, disk_total_gb) = parse_pct_bytes(lines[4], "Disk")?;

        Ok(ProbeResult {
            kernel: lines[0].to_string(),
            cpu_model: lines[1].to_string(),
            load: lines[2].to_string(),
            ram_pct,
            ram_total_gb,
            disk_pct,
            disk_total_gb,
        })
    }
}

fn parse_pct_bytes(line: &str, label: &str) -> Result<(u8, f32)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        anyhow::bail!("format {} inattendu : {:?}", label, line);
    }
    let pct: u8 = parts[0]
        .parse()
        .map_err(|e| anyhow::anyhow!("{} pct: {}", label, e))?;
    let bytes: u64 = parts[1]
        .parse()
        .map_err(|e| anyhow::anyhow!("{} bytes: {}", label, e))?;
    Ok((pct, bytes as f32 / 1_073_741_824.0))
}

// ─── Probe ────────────────────────────────────────────────────────────────────

/// Lance un diagnostic SSH non-interactif et retourne les métriques système.
///
/// Réutilise [`build_ssh_args`] pour construire tous les arguments de connexion,
/// puis ajoute le one-liner bash comme commande distante.
///
/// **Modes supportés** : `Direct` et `Jump` uniquement.
/// Le mode `Bastion` retourne une erreur immédiate car le format de connexion
/// est incompatible avec l'ajout d'une commande distante.
pub fn probe(server: &ResolvedServer, mode: ConnectionMode) -> Result<ProbeResult> {
    if mode == ConnectionMode::Bastion {
        anyhow::bail!("Le diagnostic n'est pas disponible en mode Bastion");
    }

    let mut args = build_ssh_args(server, mode, false)?;

    // La destination `user@host` est toujours le dernier argument de
    // build_ssh_args (pour Direct et Jump). On l'extrait temporairement
    // pour pouvoir insérer les options de probe avant elle.
    let destination = args
        .pop()
        .ok_or_else(|| anyhow::anyhow!("liste d'args SSH vide"))?;

    // Options probe insérées avant la destination
    args.push("-n".into()); // stdin depuis /dev/null — commande non-interactive
    args.push("-o".into());
    args.push("ConnectTimeout=10".into());
    args.push("-o".into());
    args.push("BatchMode=yes".into()); // pas d'invite interactive

    args.push(destination);
    args.push(PROBE_CMD.into()); // commande distante

    let output = Command::new("ssh").args(&args).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("SSH probe échoué : {}", stderr.trim());
    }

    ProbeResult::parse(&String::from_utf8_lossy(&output.stdout))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Sortie typique du one-liner sur un serveur Debian.
    const SAMPLE: &str = "6.1.0-28-amd64\n\
        Intel(R) Xeon(R) CPU E5-2670 0 @ 2.60GHz\n\
        0.42, 0.38, 0.31\n\
        67 16106127360\n\
        23 499963174912\n";

    #[test]
    fn parse_valid() {
        let r = ProbeResult::parse(SAMPLE).unwrap();
        assert_eq!(r.kernel, "6.1.0-28-amd64");
        assert_eq!(r.cpu_model, "Intel(R) Xeon(R) CPU E5-2670 0 @ 2.60GHz");
        assert_eq!(r.load, "0.42, 0.38, 0.31");
        assert_eq!(r.ram_pct, 67);
        assert!((r.ram_total_gb - 15.0).abs() < 1.0, "ram ~15 GB");
        assert_eq!(r.disk_pct, 23);
        assert!((r.disk_total_gb - 465.7).abs() < 1.0, "disk ~465 GB");
    }

    #[test]
    fn parse_ignores_trailing_blank_lines() {
        let with_blanks = format!("{}\n\n", SAMPLE);
        assert!(ProbeResult::parse(&with_blanks).is_ok());
    }

    #[test]
    fn parse_too_few_lines() {
        let err = ProbeResult::parse("only one line\n").unwrap_err();
        assert!(err.to_string().contains("lignes"));
    }

    #[test]
    fn parse_bad_ram_pct() {
        let bad = "6.1.0\nIntel\n0.1\nXX 16000000000\n23 500000000000\n";
        assert!(ProbeResult::parse(bad).is_err());
    }

    #[test]
    fn parse_bad_ram_bytes() {
        let bad = "6.1.0\nIntel\n0.1\n67 not_a_number\n23 500000000000\n";
        assert!(ProbeResult::parse(bad).is_err());
    }

    #[test]
    fn parse_bad_disk_pct() {
        let bad = "6.1.0\nIntel\n0.1\n67 16000000000\nXX 500000000000\n";
        assert!(ProbeResult::parse(bad).is_err());
    }

    #[test]
    fn parse_disk_single_column() {
        // Seulement un champ sur la ligne disk → erreur
        let bad = "6.1.0\nIntel\n0.1\n67 16000000000\n23\n";
        assert!(ProbeResult::parse(bad).is_err());
    }
}
