use crate::error::CozoError::DatabaseClosed;
use crate::error::Result;
use crate::value::cozo_comparator_v1;


pub struct Storage {
    pub db: Option<()>,
    path: String,
}
//
// fn make_options() -> Options {
//     let mut options = Options::default();
//
//     options.create_missing_column_families(true);
//     options.create_if_missing(true);
//     options.set_comparator("cozo_comparator_v1", cozo_comparator_v1);
//     options
// }

// #[allow(dead_code)]
// fn make_write_options(global: bool) -> WriteOptions {
//     let mut options = WriteOptions::new();
//     options.disable_wal(!global);
//     options
// }

impl Storage {
    pub fn no_storage() -> Self {
        Self { db: None, path: "".to_string() }
    }
    pub fn new(path: String) -> Result<Self> {
        unimplemented!()
        // let options = make_options();
        // let cfs = match DB::list_cf(&options, &path) {
        //     Ok(cfs) => { cfs }
        //     Err(_) => { vec![] }
        // };
        // let cfs = cfs.into_iter().map(|name| {
        //     ColumnFamilyDescriptor::new(name, make_options())
        // });
        // let db = DB::open_cf_descriptors(&options, &path, cfs)?;
        // Ok(Storage { db: Some(db), path })
    }
    pub fn delete(&mut self) -> Result<()> {
        unimplemented!();
        // drop(self.db.take());
        // DB::destroy(&make_options(), &self.path)?;
        Ok(())
    }
    pub fn put_global(&self, k: &[u8], v: &[u8]) -> Result<()> {
        // let db = self.db.as_ref().ok_or(DatabaseClosed)?;
        // db.put(k, v)?;
        unimplemented!();
        Ok(())
    }
    pub fn create_table(&mut self, name: &str, _global: bool) -> Result<()> {
        unimplemented!();
        // let db = self.db.as_mut().ok_or(DatabaseClosed)?;
        // db.create_cf(name, &make_options())?;
        Ok(())
    }
    pub fn drop_table(&mut self, name: &str, _global: bool) -> Result<()> {
        unimplemented!();
        // let db = self.db.as_mut().ok_or(DatabaseClosed)?;
        // db.drop_cf(name)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::str::from_utf8;
    use crate::typing::BaseType::String;
    use super::*;

    #[test]
    fn import() {
        use cozo_rocks_sys::*;

        let options = new_options();
        options.increase_parallelism();
        options.optimize_level_style_compaction();
        options.set_create_if_missing(true);
        let db = open_db(&options, "xxyyzz");
        let mut status = Status::new();
        db.put("A key".as_bytes(), "A motherfucking value!!! 👋👋👋".as_bytes(), &mut status);
        let val = db.get("A key".as_bytes());
        let val = val.as_bytes();
        println!("{:?} {}", status, from_utf8(val).unwrap());
    }
}