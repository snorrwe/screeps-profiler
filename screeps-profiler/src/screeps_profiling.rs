use crate::*;
use screeps::raw_memory;
use std::collections::HashMap;

static mut TABLE: *mut ProfileTable = std::ptr::null_mut();
static mut IDS: *mut HashMap<&'static str, ProfileId> = std::ptr::null_mut();

lazy_static! {
    static ref CACHE: (ProfileTable, HashMap<&'static str, ProfileId>) = {
        unsafe {
            let mut table = ProfileTable::new();
            let mut ids = HashMap::new();
            TABLE = &mut table as *mut _;
            IDS = &mut ids as *mut _;
            (table, ids)
        }
    };
}

pub unsafe fn create_sentinel(name: &'static str) -> ProfileSentinel<fn() -> f64> {
    &CACHE; // Init
    let id = (*IDS)
        .entry(name)
        .or_insert_with(|| (*TABLE).add_entity(name.to_owned()));

    new_sentinel(*id, &mut *TABLE)
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
            .and_then(|string| serde_json::from_str(&string).ok())
            .unwrap_or_else(|| {
                let mut result = Self::default();
                result.memory_segment = memory_segment;
                result
            })
    }
}

impl Drop for RawMemoryProfiler {
    fn drop(&mut self) {
        let table = CACHE.0.clone();
        self.data.push(table);
        let data = serde_json::to_string(self).expect("Failed to serialize RawMemoryProfiler");
        raw_memory::set_segment(self.memory_segment as u32, data.as_str());
    }
}

