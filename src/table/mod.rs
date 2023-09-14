use crate::prelude::{miette, Result, WriteAs, CSV};
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

type CellKey = usize;
type Header<'a> = HashMap<&'a str, usize>;

/// # Memory Management
///
/// Operations altering the structure of the table do not immediately drop data when no longer
/// used.
/// Instead, once the number of data cells _exceeds_ a threshold, a manual cleanup phase is
/// triggered, dropping all unused data.
/// The threshold is currently set to `5*(rows.len())*(cols.len())`, but is subject to change.
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
        P: FnMut(&T) -> bool,
    {
        let mut e = None;
        let col = &col;
        let new = self.filter_rows(|row| match row.get(col) {
            Ok(t) => pred(t),
            Err(err) => {
                e = Some(err);
                true
            }
        });

        match e {
            Some(e) => Err(e),
            None => Ok(new),
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
        P: FnMut(&str) -> bool,
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

    pub fn map<C, F>(self, col: C, mut f: F) -> Result<Self>
    where
        C: Column,
        F: FnMut(Row<T>, &T) -> T,
    {
        let Self {
            mut cells,
            rows,
            cols,
        } = self;
        let hdr = build_header(&cols);
        let i = col
            .get(&hdr)
            .ok_or_else(|| miette!("could not find column {col} in table"))?;

        for row in &rows {
            let t = f(Row::new(&hdr, row, &cells), &cells[i]);
            cells[i] = t;
        }

        Ok(Self { cells, rows, cols })
    }

    pub fn append<H, F>(self, header: H, mut f: F) -> Self
    where
        H: Into<String>,
        F: FnMut(Row<T>) -> T,
    {
        let Self {
            mut cells,
            mut rows,
            mut cols,
        } = self;
        let hdr = build_header(&cols);

        for row in &mut rows {
            let t = f(Row::new(&hdr, row, &cells));
            let k = cells.len();
            cells.push(t);
            row.push(k);
        }

        cols.push(header.into());

        Self { cells, rows, cols }
    }

    pub fn rename<M, C, N>(mut self, map: M) -> Result<Self>
    where
        M: IntoIterator<Item = (C, N)>,
        C: Column,
        N: Into<String>,
    {
        let hdr = build_header(&self.cols);
        let ren: Vec<_> = map
            .into_iter()
            .map(|(c, n)| {
                c.get(&hdr)
                    .ok_or_else(|| miette!("could not find column {c} in table"))
                    .map(|i| (i, n.into()))
            })
            .collect::<Result<_>>()?;

        for (i, n) in ren {
            self.cols[i] = n;
        }

        Ok(self)
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

    pub fn maybe_consolidate(mut self) -> Self {
        let threshold = 5 * self.rows.len() * self.cols.len();
        if self.cells.len() <= threshold {
            return self;
        }

        let used_idxs = self
            .rows
            .iter()
            .flatten()
            .copied()
            .collect::<HashSet<CellKey>>();

        let mut i = 0;
        self.cells.retain(|_| {
            let x = used_idxs.contains(&i);
            i += 1;
            x
        });

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

    pub fn get<C: Column>(&self, col: C) -> Result<&T> {
        self.try_get(&col)
            .ok_or_else(|| miette!("could not find column {col} in table"))
    }

    pub fn try_get<C: Column>(&self, col: &C) -> Option<&T> {
        col.get(self.hdr)
            .and_then(|i| self.row.get(i))
            .and_then(|&i| self.cells.get(i))
    }
}

pub struct Col;

pub trait Column: fmt::Display {
    fn find(&self, cols: &[String]) -> Option<usize>;
    fn get(&self, hdr: &Header) -> Option<usize>;
}

impl<C: Column> Column for &C {
    fn get(&self, hdr: &Header) -> Option<usize> {
        <C as Column>::get(&self, hdr)
    }
    fn find(&self, hdr: &[String]) -> Option<usize> {
        <C as Column>::find(&self, hdr)
    }
}

impl Column for str {
    fn get(&self, hdr: &Header) -> Option<usize> {
        hdr.get(self).copied()
    }

    fn find(&self, cols: &[String]) -> Option<usize> {
        cols.iter()
            .enumerate()
            .find_map(|(i, s)| s.eq(self).then_some(i))
    }
}

impl Column for String {
    fn get(&self, hdr: &Header) -> Option<usize> {
        <str as Column>::get(self.as_str(), hdr)
    }
    fn find(&self, hdr: &[String]) -> Option<usize> {
        <str as Column>::find(self.as_str(), hdr)
    }
}

impl Column for usize {
    fn get(&self, hdr: &Header) -> Option<usize> {
        (*self < hdr.len()).then_some(*self)
    }

    fn find(&self, cols: &[String]) -> Option<usize> {
        (*self < cols.len()).then_some(*self)
    }
}
