#[macro_use]
extern crate serde;
#[cfg(feature = "screeps")]
#[macro_use]
extern crate lazy_static;
#[cfg(feature = "screeps")]
extern crate screeps;

#[cfg(feature = "screeps")]
pub mod screeps_profiling {
    use super::*;
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

    // TODO:
    //
    // Object to save the table into RawMemory
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfileTable {
    labels: Vec<String>,
    data: Vec<ProfileRow>,
}

impl ProfileTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_entity(&mut self, name: String) -> ProfileId {
        let id = self.data.len();
        self.labels.push(name);
        self.data.push(ProfileRow::default());
        ProfileId { id }
    }

    pub fn get_label<'a>(&'a self, id: ProfileId) -> Option<&'a String> {
        self.labels.get(id.id)
    }

    pub fn get_data<'a>(&'a self, id: ProfileId) -> Option<&'a ProfileRow> {
        self.data.get(id.id)
    }

    pub fn get_data_mut<'a>(&'a mut self, id: ProfileId) -> Option<&'a mut ProfileRow> {
        self.data.get_mut(id.id)
    }
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub struct ProfileId {
    pub id: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfileRow {
    pub cpu_per_call: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct ProfileSentinel<TCpuFun>
where
    TCpuFun: FnMut() -> f64,
{
    id: ProfileId,
    table: *mut ProfileTable,

    cpu_at_start: f64,

    get_cpu: TCpuFun,
}

impl<T> ProfileSentinel<T>
where
    T: FnMut() -> f64,
{
    pub fn new(id: ProfileId, table: &mut ProfileTable, mut get_cpu: T) -> Self {
        let cpu = get_cpu();

        Self {
            cpu_at_start: cpu,
            table: table as *mut _,
            get_cpu,
            id,
        }
    }
}

impl<T> Drop for ProfileSentinel<T>
where
    T: FnMut() -> f64,
{
    fn drop(&mut self) {
        let delta = (self.get_cpu)() - self.cpu_at_start;

        let row = unsafe {
            (*self.table).get_data_mut(self.id).expect(&format!(
                "Expected a profile row to be available by id {:?}",
                self.id
            ))
        };

        row.cpu_per_call.push(delta);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut table = ProfileTable::new();

        let call_one = table.add_entity("One".to_owned());
        let call_two = table.add_entity("Two".to_owned());
        let call_zero = table.add_entity("Zero".to_owned());

        let mut cpu = 1.;
        let get_cpu = move || {
            cpu += 2.0;
            cpu
        };

        {
            let _sentinel = ProfileSentinel::new(call_one, &mut table, get_cpu);
        }

        {
            let _sentinel = ProfileSentinel::new(call_two, &mut table, get_cpu);
            let _sentinel = ProfileSentinel::new(call_two, &mut table, get_cpu);
        }

        println!("{:#?}", table);

        assert_eq!(
            table.get_data(call_one).map(|x| x.cpu_per_call.len()),
            Some(1)
        );
        assert_eq!(
            table.get_data(call_two).map(|x| x.cpu_per_call.len()),
            Some(2)
        );

        assert_eq!(
            table.get_data(call_zero).map(|x| x.cpu_per_call.len()),
            Some(0)
        );

        let cpu = table.get_data(call_one).map(|x| x.cpu_per_call[0]).unwrap();
        assert!((2.0 - cpu).abs() < 0.0003);

        let cpu = table.get_data(call_two).map(|x| x.cpu_per_call[0]).unwrap();
        assert!((2.0 - cpu).abs() < 0.0003);
        let cpu = table.get_data(call_two).map(|x| x.cpu_per_call[1]).unwrap();
        assert!((2.0 - cpu).abs() < 0.0003);
    }
}
