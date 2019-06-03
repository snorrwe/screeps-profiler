#[macro_use]
extern crate serde;
#[macro_use]
extern crate log;
#[cfg(feature = "screeps")]
#[macro_use]
extern crate lazy_static;
#[cfg(feature = "screeps")]
extern crate screeps;
#[cfg(feature = "screeps")]
extern crate serde_json;

#[cfg(feature = "screeps")]
pub mod screeps_profiling;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfileTable {
    labels: Vec<String>,
    data: Vec<ProfileRow>,
}

impl ProfileTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.labels.clear();
        self.data.clear();
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

