//! Persistance NVS : configuration (mapping, turbo, macros, LEDs...) et
//! statistiques d'utilisation.

use controller_core::config::Config;
use controller_core::stats::Stats;
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};

const NAMESPACE: &str = "nexus_one";
const KEY_CONFIG: &str = "config";
const KEY_STATS: &str = "stats";

pub struct Storage {
    nvs: EspNvs<NvsDefault>,
}

impl Storage {
    pub fn new(partition: EspNvsPartition<NvsDefault>) -> anyhow::Result<Self> {
        Ok(Self { nvs: EspNvs::new(partition, NAMESPACE, true)? })
    }

    pub fn load_config(&self) -> Config {
        let mut buf = [0u8; 1024];
        match self.nvs.get_blob(KEY_CONFIG, &mut buf) {
            Ok(Some(bytes)) => match Config::from_bytes(bytes) {
                Ok(cfg) => cfg,
                Err(_) => {
                    log::warn!("config NVS illisible (version ?), retour aux réglages d'usine");
                    Config::default()
                }
            },
            _ => Config::default(),
        }
    }

    pub fn save_config(&mut self, config: &Config) -> anyhow::Result<()> {
        let mut buf = [0u8; 1024];
        let bytes = config.to_bytes(&mut buf).map_err(|e| anyhow::anyhow!("encode: {e:?}"))?;
        self.nvs.set_blob(KEY_CONFIG, bytes)?;
        Ok(())
    }

    pub fn load_stats(&self) -> Stats {
        let mut buf = [0u8; 256];
        match self.nvs.get_blob(KEY_STATS, &mut buf) {
            Ok(Some(bytes)) => postcard::from_bytes(bytes).unwrap_or_default(),
            _ => Stats::default(),
        }
    }

    pub fn save_stats(&mut self, stats: &Stats) -> anyhow::Result<()> {
        let mut buf = [0u8; 256];
        let bytes =
            postcard::to_slice(stats, &mut buf).map_err(|e| anyhow::anyhow!("encode: {e:?}"))?;
        self.nvs.set_blob(KEY_STATS, bytes)?;
        Ok(())
    }

    pub fn factory_reset(&mut self) -> anyhow::Result<()> {
        let _ = self.nvs.remove(KEY_CONFIG);
        let _ = self.nvs.remove(KEY_STATS);
        Ok(())
    }
}
