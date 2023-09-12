use crate::prelude::{Result, WriteAs, CSV};



pub struct Table;

/// # Filtering
impl Table {
    pub fn filter<P>(&self, pred: P) -> Self {
        todo!()
    }

    pub fn filter_rows<P>(&self, pred: P) -> Self {
        todo!()
    }

    pub fn filter_cols<P>(&self, pred: P) -> Self {
        todo!()
    }
}

/// # Column Manipulations
impl Table {
    pub fn rename<M>(&self, map: M) -> Self {
        todo!()
    }

    pub fn typify<M>(&self, map: M) -> Self {
        todo!()
    }
}

impl WriteAs<CSV, ()> for Table {
    fn write_as(&self, fmt: CSV, wtr: &mut dyn std::io::Write) -> Result<()> {
        todo!()
    }
}

pub struct Row;

impl Row {
    pub fn get<C, T>(&self, col: C) -> Result<T> {
        todo!()
    }

    pub fn try_get<C, T>(&self, col: C) -> Option<T> {
        todo!()
    }
}
