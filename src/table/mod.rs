use crate::prelude::{miette, Result, WriteAs, CSV};
use std::{collections::HashMap, fmt};

type CellKey = usize;
type Header<'a> = HashMap<&'a str, usize>;

#[derive(Clone)]
pub struct Table<T> {
    cells: Vec<T>,
    rows: Vec<Vec<CellKey>>,
    cols: Vec<String>,
}

impl<T> Table<T> {
    pub fn new() -> Self {
        Self {
            cells: Default::default(),
            rows: Default::default(),
            cols: Default::default(),
        }
    }

    pub fn get_row(&self, row_index: usize) -> Result<Row<T>> {
        todo!()
    }

    pub fn try_get_row(&self, row_index: usize) -> Option<Row<T>> {
        todo!()
    }

    pub fn get_col<C>(&self, col: C) -> Result<Col> {
        todo!()
    }

    pub fn try_get_col<C>(&self, col: C) -> Option<Col> {
        todo!()
    }

    pub fn filter<C, P>(self, col: C, mut pred: P) -> Result<Self> 
    where
    C: Column,
    P: FnMut(&T) -> bool
    {
        let mut e = None;
        let col = &col;
        let new = self.filter_rows(|row| {
            match row.get(col) {
                Ok(t) => pred(t),
                Err(err) => {
                    e = Some(err);
                    true
                }
            }
        });

        match e {
            Some(e) => Err(e),
            None => Ok(new)
        }
    }

    pub fn filter_rows<P>(self, mut pred: P) -> Self
    where
        P: FnMut(Row<T>) -> bool,
    {
        let Self {
            cells,
            mut rows,
            cols,
        } = self;
        let hdr = build_header(&cols);
        rows.retain(|row| pred(Row::new(&hdr, &row, &cells)));

        Self { cells, rows, cols }.maybe_consolidate()
    }

    pub fn filter_cols<P>(mut self, mut pred: P) -> Self 
    where
    P: FnMut(&str) -> bool
    {
        let mut rm = Vec::new();
        let mut i = 0;
        self.cols.retain(|h| {
            let x = pred(h);
            if !x {
                rm.push(i);
            }
            i += 1;
            x
        });

        rm.reverse(); // reverse to remove stably
        for row in &mut self.rows {
            for i in &rm {
                row.remove(*i);
            }
            debug_assert_eq!(row.len(), self.cols.len());
        }

        self.maybe_consolidate()
    }

    pub fn insert<R>(self, row_index: usize, row: R) -> Self {
        todo!()
    }

    pub fn map<C, F>(self, col: C, f: F) -> Self {
        todo!()
    }

    pub fn append<H, F>(self, header: H, f: F) -> Self {
        todo!()
    }

    pub fn rename<M>(self, map: M) -> Self {
        todo!()
    }

    pub fn typify<M>(self, map: M) -> Self {
        todo!()
    }

    pub fn sort_rows<F>(self, f: F) -> Self {
        todo!()
    }

    pub fn sort_cols<F>(self, f: F) -> Self {
        todo!()
    }

    pub fn pick<M>(self, trail: bool, map: M) -> Self {
        todo!()
    }

    pub fn display<S: AsRef<str>>(self, style: S) -> comfy_table::Table {
        todo!()
    }

    pub fn maybe_consolidate(self) -> Self {
        self
    }
}

fn build_header(cols: &[String]) -> Header {
    cols.iter()
        .enumerate()
        .map(|(i, h)| (h.as_str(), i))
        .collect()
}

impl<T> WriteAs<CSV, ()> for Table<T> {
    fn write_as(&self, fmt: CSV, wtr: &mut dyn std::io::Write) -> Result<()> {
        todo!()
    }
}

pub struct Row<'a, T> {
    hdr: &'a Header<'a>,
    row: &'a [usize],
    cells: &'a [T],
}

impl<'a, T> Row<'a, T> {
    fn new(hdr: &'a Header<'a>, row: &'a [usize], cells: &'a [T]) -> Self {
        Self { hdr, row, cells }
    }

    pub fn get<C: Column>(&self, col: C) -> Result<&T>
    {
        self.try_get(&col)
            .ok_or_else(|| miette!("could not find column {} in table", &col))
    }

    pub fn try_get<C: Column>(&self, col: &C) -> Option<&T> {
        col.idx(self.hdr)
            .and_then(|i| self.row.get(i))
            .and_then(|&i| self.cells.get(i))
    }
}

pub struct Col;

trait Column: fmt::Display {
    fn idx(&self, hdr: &Header) -> Option<usize>;
}

impl<C: Column> Column for &C {
    fn idx(&self, hdr: &Header) -> Option<usize> {
        <C as Column>::idx(&self, hdr)
    }
}

impl Column for str {
    fn idx(&self, hdr: &Header) -> Option<usize> {
        hdr.get(self).copied()
    }
}

impl Column for String {
    fn idx(&self, hdr: &Header) -> Option<usize> {
        hdr.get(self.as_str()).copied()
    }
}

impl Column for usize {
    fn idx(&self, hdr: &Header) -> Option<usize> {
        (*self < hdr.len()).then_some(*self)
    }
}
