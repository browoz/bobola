#[path = "../resources.rs"]
#[allow(dead_code)]
mod resources;

fn main() -> Result<(), String> {
    #[cfg(windows)]
    {
        let roots: Vec<u32> = std::env::args()
            .skip(1)
            .map(|value| {
                value
                    .parse()
                    .map_err(|_| "Expected process IDs.".to_string())
            })
            .collect::<Result<_, _>>()?;
        if roots.is_empty() {
            return Err("Supply at least one process ID.".into());
        }
        println!(
            "{}",
            serde_json::to_string(&resources::sample_roots(&roots)?).map_err(|e| e.to_string())?
        );
        Ok(())
    }
    #[cfg(not(windows))]
    Err("Windows only.".into())
}
