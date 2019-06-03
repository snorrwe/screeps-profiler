use crate::*;
use screeps::raw_memory;
use std::collections::HashMap;
use std::sync::Mutex;

#[macro_export]
macro_rules! profile {
    ($name: expr) => {
        let _sentinel = unsafe {
            use screeps_profiler::screeps_profiling::create_sentinel;
            let name = concat!(module_path!(), "::", $name);
            create_sentinel(&name)
        };
    };
}

lazy_static! {
    static ref TABLE: Mutex<ProfileTable> = Mutex::new(ProfileTable::new());
    static ref IDS: Mutex<HashMap<&'static str, ProfileId>> = Mutex::new(HashMap::new());
}

pub unsafe fn create_sentinel(name: &'static str) -> ProfileSentinel<fn() -> f64> {
    let mut table = TABLE.lock().unwrap();
    let mut ids = IDS.lock().unwrap();
    let id = ids
        .entry(name)
        .or_insert_with(|| table.add_entity(name.to_owned()));

    new_sentinel(*id, &mut table)
}

pub fn new_sentinel(id: ProfileId, table: &mut ProfileTable) -> ProfileSentinel<fn() -> f64> {
    ProfileSentinel::new(id, table, screeps::game::cpu::get_used)
}

#[derive(Serialize, Deserialize, Default)]
pub struct RawMemoryProfiler {
    /// Where to save this state when dropping
    /// Defaults to 0
    #[serde(skip_serializing)]
    #[serde(default)]
    pub memory_segment: u8,
    data: Vec<ProfileTable>,
}

impl RawMemoryProfiler {
    pub fn read_from_segment_or_default(memory_segment: u8) -> Self {
        raw_memory::get_segment(memory_segment as u32)
            .and_then(|string| {
                serde_json::from_str(&string)
                    .ok()
                    .map(|data: Vec<ProfileTable>| Self {
                        data,
                        memory_segment: memory_segment,
                    })
            })
            .or_else(|| {
                let mut result = Self::default();
                result.memory_segment = memory_segment;
                Some(result)
            })
            .unwrap()
    }
}

impl Drop for RawMemoryProfiler {
    fn drop(&mut self) {
        let table = TABLE.lock().unwrap().clone();
        self.data.push(table);
        let data =
            serde_json::to_string(&self.data).expect("Failed to serialize RawMemoryProfiler");

        TABLE.lock().unwrap().clear();
        IDS.lock().unwrap().clear();

        debug!("Saving RawMemoryProfiler {:?}", data);

        raw_memory::set_segment(self.memory_segment as u32, data.as_str());
    }
}

