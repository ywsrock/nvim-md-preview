//! WSL で動いているときに、Windows 側で使われているポートを調べる
//!
//! WSL2（NAT モード）の localhost は Windows と別なので、Windows 側のプレビューと同じポートでも
//! 待ち受けられてしまう。ところがブラウザは Windows で動くので、127.0.0.1:<port> は Windows 側の
//! サーバーにつながる。これを避けるため、Windows 側で待ち受け中のポートは使わない

use std::{collections::HashSet, time::Duration};
use tokio::process::Command;

/// netstat.exe を待つ時間の上限
const TIMEOUT: Duration = Duration::from_secs(3);

/// WSL の中で動いているか
pub fn is_wsl() -> bool {
    if std::env::var_os("WSL_DISTRO_NAME").is_some() {
        return true;
    }
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| {
            let s = s.to_lowercase();
            s.contains("microsoft") || s.contains("wsl")
        })
        .unwrap_or(false)
}

/// Windows 側で TCP を待ち受けているポート。WSL でないときや netstat.exe が使えないときは空
pub async fn windows_listening_ports() -> HashSet<u16> {
    if !is_wsl() {
        return HashSet::new();
    }
    // Windows の PATH を WSL に通していない環境（appendWindowsPath = false）でも使えるように、既定の場所も試す
    for exe in ["netstat.exe", "/mnt/c/Windows/System32/netstat.exe"] {
        let mut cmd = Command::new(exe);
        cmd.arg("-an").kill_on_drop(true).stdin(std::process::Stdio::null());
        if let Ok(Ok(out)) = tokio::time::timeout(TIMEOUT, cmd.output()).await {
            if out.status.success() {
                return parse_listening(&String::from_utf8_lossy(&out.stdout));
            }
        }
    }
    HashSet::new()
}

/// `netstat -an` の出力から、待ち受け中の TCP のポートを取り出す
/// 状態の列（LISTENING）は Windows の言語によって訳されるので使わず、
/// 相手のアドレスが 0.0.0.0:0 / [::]:0 の行を待ち受けとみなす
pub fn parse_listening(output: &str) -> HashSet<u16> {
    output
        .lines()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 3 || !cols[0].eq_ignore_ascii_case("tcp") {
                return None;
            }
            if cols[2] != "0.0.0.0:0" && cols[2] != "[::]:0" {
                return None;
            }
            cols[1].rsplit_once(':')?.1.parse().ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_listening_ports() {
        let out = "
Active Connections

  Proto  Local Address          Foreign Address        State
  TCP    0.0.0.0:135            0.0.0.0:0              LISTENING
  TCP    127.0.0.1:9876         0.0.0.0:0              LISTENING
  TCP    127.0.0.1:9876         127.0.0.1:50123        ESTABLISHED
  TCP    127.0.0.1:50123        127.0.0.1:9876         ESTABLISHED
  TCP    [::]:445               [::]:0                 LISTENING
  TCP    [::1]:9877             [::]:0                 ABHÖREN
  UDP    0.0.0.0:5353           *:*
";
        let ports = parse_listening(out);
        let mut v: Vec<u16> = ports.into_iter().collect();
        v.sort();
        assert_eq!(v, vec![135, 445, 9876, 9877]);
    }

    #[test]
    fn ignores_unrelated_output() {
        assert!(parse_listening("").is_empty());
        assert!(parse_listening("アクティブな接続\n\n  プロトコル  ローカル アドレス\n").is_empty());
    }
}
