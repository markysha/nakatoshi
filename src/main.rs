use bitcoin::secp256k1::Secp256k1;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::iter::ParallelIterator;
use serde_json::json;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;

mod address;
mod cli;

use address::BitcoinAddress;

fn main() {
    let matches = cli::prompt().get_matches();
    let secp = Secp256k1::new();

    let is_case_sensitive = matches.get_flag("case-sensitive");
    let is_bech32 = matches.get_flag("bech32");
    let is_compressed = !matches.get_flag("uncompressed");

    let num_threads = matches
        .get_one::<String>("threads")
        .and_then(|num_threads| num_threads.parse().ok())
        .unwrap_or_else(num_cpus::get);

    let regexes = match matches.get_one::<String>("regex") {
        Some(regex) => vec![regex.to_owned()],
        None => {
            let file_name = matches.get_one::<String>("input-file").unwrap();
            get_regexes_from_file(file_name)
        }
    };

    let rayon_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .expect("Failed to create thread pool");

    let progress = ProgressBar::new_spinner();
    let template = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {pos} attempts")
        .unwrap();
    progress.set_style(template);
    // progress.set_draw_rate(10);

    let bitcoin_address: BitcoinAddress = rayon_pool.install(|| {
        rayon::iter::repeat(BitcoinAddress::new)
            .inspect(|_| progress.inc(1))
            .map(|create| create(&secp, is_compressed, is_bech32))
            .find_any(|address| address.matches_with_any(&regexes, is_case_sensitive))
            .expect("Failed to find Bitcoin address match")
    });

    let attempts = progress.position();
    progress.finish_and_clear();

    let result = json!({
        "private_key": bitcoin_address.private_key.to_string(),
        "public_key": bitcoin_address.public_key.to_string(),
        "address": bitcoin_address.address.to_string(),
        "attempts": attempts
    });

    print!("{}", result);
}

fn get_regexes_from_file(file_name: &str) -> Vec<String> {
    let file = fs::File::open(file_name).unwrap();
    let buffer = BufReader::new(file);

    let mut regexes = buffer
        .lines()
        .map(|line| line.expect("Failed to read Bitcoin address pattern from input file"))
        .collect::<Vec<String>>();

    regexes.sort_by_key(|a| a.len());
    regexes
}

#[cfg(test)]
mod tests {
    use crate::address::BitcoinAddress;
    use bitcoin::secp256k1::Secp256k1;

    #[test]
    fn create_compressed_bitcoin_public_key() {
        let secp = Secp256k1::new();
        let is_bech32 = false;
        let is_compressed = true;
        let bitcoin_address = BitcoinAddress::new(&secp, is_compressed, is_bech32);

        let actual = bitcoin_address.public_key.to_string().len();
        let expected = 66;

        assert_eq!(actual, expected);
    }

    #[test]
    fn create_uncompressed_bitcoin_public_key() {
        let secp = Secp256k1::new();
        let is_bech32 = false;
        let is_compressed = false;
        let bitcoin_address = BitcoinAddress::new(&secp, is_compressed, is_bech32);

        let actual = bitcoin_address.public_key.to_string().len();
        let expected = 130;

        assert_eq!(actual, expected);
    }

    #[test]
    fn create_bech32_address() {
        let secp = Secp256k1::new();
        let is_bech32 = true;
        let is_compressed = true;
        let bitcoin_address = BitcoinAddress::new(&secp, is_compressed, is_bech32);
        let address = bitcoin_address.address.to_string();

        assert!(address.starts_with("bc1q"));
    }

    #[test]
    fn create_bitcoin_private_key() {
        let secp = Secp256k1::new();
        let is_bech32 = false;
        let is_compressed = true;
        let bitcoin_address = BitcoinAddress::new(&secp, is_compressed, is_bech32);

        let actual = bitcoin_address.private_key.to_string().len();
        let expected = 52;

        assert_eq!(actual, expected);
    }

    #[test]
    fn create_bitcoin_address() {
        let secp = Secp256k1::new();
        let is_bech32 = false;
        let is_compressed = true;
        let bitcoin_address = BitcoinAddress::new(&secp, is_compressed, is_bech32);

        let actual = bitcoin_address.address.to_string().len();
        let expected = 34;

        assert_eq!(actual, expected);
    }
}
