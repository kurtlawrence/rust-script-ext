use miette::IntoDiagnostic;

use crate::prelude::{miette, Read, Result, WriteAs, CSV};
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

type CellKey = usize;
type Header<'a> = HashMap<&'a str, usize>;

/// A general table implementation with an ergonomic API for transforming tables.
///
/// This structure is meant for use cases where the _contents_ of the table is not strongly
/// structured, or where a transformation is to be applied before persisting again.
/// In contrast to [`crate::prelude::CsvReader`], rather than needing an inner `T: Deserliaze`,
/// each cell will contain `T`. Much of the API is around _restructuring_ the data, so filtering
/// rows/columns, sorting, rearranging columns, etc.
///
/// # Memory Usage
///
/// The table stores all cell data `T` in a single vector.
/// It also stores rows as a vector of `Vec<usize>`.
/// The _minimum_ memory footprint is thus:
///
/// ```plaintext
/// Rn = number of rows
/// Cn = number of cols
/// T  = size_of::<T>
///
/// Rn * Cn * T   # cell data
/// + Rn * Cn * 8 # usize keys into cell data
/// + Cn * 24     # headers stored as strings
/// ```
///
/// Operations altering the structure of the table do not immediately drop data when no longer
/// used.
/// Instead, once the number of data cells _exceeds_ a threshold, a manual cleanup phase is
/// triggered, dropping all unused data.
/// The threshold is currently set to `5*(rows.len())*(cols.len())`, but is subject to change.
#[derive(Clone)]
pub struct Table<T> {
    cells: Vec<T>,
    rows: Vec<Vec<CellKey>>,
    cols: Vec<String>,
}

impl<T> Table<T> {
    /// Construct a new, empty, table.
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

    /// Filter rows where a specific column contents matches a predicate.
    ///
    /// # Example
    /// ```rust
    /// use rust_script_ext::prelude::*;
    /// let x = Table::from_csv(
    ///     "city,pop\nBrisbane,100000\nSydney,200000\n".as_bytes()
    /// ).unwrap()
    /// .filter("city", |s| s.starts_with('B')).unwrap()
    /// .display()
    /// .to_string();
    ///
    /// assert_eq!(&x, "\
    /// +----------+--------+
    /// | city     | pop    |
    /// |----------+--------|
    /// | Brisbane | 100000 |
    /// +----------+--------+");
    /// ```
    pub fn filter<C, P>(self, col: C, mut pred: P) -> Result<Self>
    where
        C: Column,
        P: FnMut(&T) -> bool,
    {
        let col = &col;
        self.filter_rows(|row| row.get(col).map(&mut pred))
    }

    /// Filter rows based on a predicate.
    ///
    /// # Example
    /// ```rust
    /// use rust_script_ext::prelude::*;
    /// let x = Table::from_csv(
    ///     "city,pop\nBrisbane,100000\nSydney,200000\n".as_bytes()
    /// ).unwrap()
    /// .filter_rows(|row| row.get("city").map(|x| x.eq("Brisbane"))).unwrap()
    /// .display()
    /// .to_string();
    ///
    /// assert_eq!(&x, "\
    /// +----------+--------+
    /// | city     | pop    |
    /// |----------+--------|
    /// | Brisbane | 100000 |
    /// +----------+--------+");
    /// ```
    pub fn filter_rows<P>(self, mut pred: P) -> Result<Self>
    where
        P: FnMut(Row<T>) -> Result<bool>,
    {
        let Self {
            cells,
            mut rows,
            cols,
        } = self;
        let hdr = build_header(&cols);

        let mut e = None;
        rows.retain(|row| match pred(Row::new(&hdr, &row, &cells)) {
            Ok(x) => x,
            Err(err) => {
                e = Some(err);
                true
            }
        });

        match e {
            Some(e) => Err(e),
            None => Ok(Self { cells, rows, cols }.maybe_consolidate()),
        }
    }

    /// Filter columns that match the predicate.
    ///
    /// # Example
    /// ```rust
    /// use rust_script_ext::prelude::*;
    /// let x = Table::from_csv(
    ///     "city,pop\nBrisbane,100000\nSydney,200000\n".as_bytes()
    /// ).unwrap()
    /// .filter_cols(|s| s == "city")
    /// .display()
    /// .to_string();
    ///
    /// assert_eq!(&x, "\
    /// +----------+
    /// | city     |
    /// |----------|
    /// | Brisbane |
    /// |----------|
    /// | Sydney   |
    /// +----------+");
    /// ```
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

    /// Map the contents of a column.
    ///
    /// # Example
    /// ```rust
    /// use rust_script_ext::prelude::*;
    /// let x = Table::from_csv(
    ///     "city,pop\nBrisbane,100000\nSydney,200000\n".as_bytes()
    /// ).unwrap()
    /// .map_col("city", |row, c| c.to_lowercase()).unwrap()
    /// .display()
    /// .to_string();

    /// assert_eq!(&x, "\
    /// +----------+--------+
    /// | city     | pop    |
    /// |----------+--------|
    /// | brisbane | 100000 |
    /// |----------+--------|
    /// | sydney   | 200000 |
    /// +----------+--------+");
    /// ```
    pub fn map_col<C, F>(self, col: C, mut f: F) -> Result<Self>
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
            let j = row[i];
            let t = f(Row::new(&hdr, row, &cells), &cells[j]);
            cells[j] = t;
        }

        Ok(Self { cells, rows, cols })
    }

    /// Append a column to the end of the table.
    ///
    /// # Example
    /// ```rust
    /// use rust_script_ext::prelude::*;
    /// let x = Table::from_csv(
    ///     "city,pop\nBrisbane,100000\nSydney,200000\n".as_bytes()
    /// ).unwrap()
    /// .append("country", |row| "Australia".to_string())
    /// .display()
    /// .to_string();

    /// assert_eq!(&x, "\
    /// +----------+--------+-----------+
    /// | city     | pop    | country   |
    /// |----------+--------+-----------|
    /// | Brisbane | 100000 | Australia |
    /// |----------+--------+-----------|
    /// | Sydney   | 200000 | Australia |
    /// +----------+--------+-----------+");
    /// ```
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

    /// Rename columns by mapping the name to another.
    ///
    /// # Example
    /// ```rust
    /// use rust_script_ext::prelude::*;
    /// let x = Table::from_csv(
    ///     "city,pop\nBrisbane,100000\nSydney,200000\n".as_bytes()
    /// ).unwrap()
    /// .rename([
    ///     ("city", "City"),
    ///     ("pop", "Population")
    /// ]).unwrap()
    /// .display()
    /// .to_string();

    /// assert_eq!(&x, "\
    /// +----------+------------+
    /// | City     | Population |
    /// |----------+------------|
    /// | Brisbane | 100000     |
    /// |----------+------------|
    /// | Sydney   | 200000     |
    /// +----------+------------+");
    /// ```
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

    /// Reorders columns by picking out the specific columns _in order_.
    ///
    /// # Example
    /// ```rust
    /// use rust_script_ext::prelude::*;
    /// let x = Table::from_csv(
    ///     "city,pop\nBrisbane,100000\nSydney,200000\n".as_bytes()
    /// ).unwrap()
    /// .pick(false, ["pop"]).unwrap()
    /// .display()
    /// .to_string();

    /// assert_eq!(&x, "\
    /// +--------+
    /// | pop    |
    /// |--------|
    /// | 100000 |
    /// |--------|
    /// | 200000 |
    /// +--------+");
    /// ```
    pub fn pick<M, C>(mut self, trail: bool, map: M) -> Result<Self>
    where
        M: IntoIterator<Item = C>,
        C: Column,
    {
        let hdr = build_header(&self.cols);

        let mut cols = Vec::new();
        let mut inds = Vec::new();
        for c in map {
            let i = c
                .get(&hdr)
                .ok_or_else(|| miette!("could not find column {c} in table"))?;

            cols.push(self.cols[i].clone());
            inds.push(i);
        }

        std::mem::swap(&mut self.cols, &mut cols);

        if trail {
            for (i, c) in cols.into_iter().enumerate() {
                if !inds.contains(&i) {
                    self.cols.push(c);
                    inds.push(i);
                }
            }
        }

        for r in &mut self.rows {
            let mut r_ = Vec::with_capacity(inds.len());
            for i in &inds {
                r_.push(r[*i]);
            }

            *r = r_;
        }

        Ok(self.maybe_consolidate())
    }

    /// Convert this table into [`comfy_table::Table`], which can be displayed prettily.
    ///
    /// See [`comfy_table::presets`] for a bunch of styles.
    ///
    /// # Example
    /// ```rust
    /// use rust_script_ext::prelude::*;
    /// let t = Table::from_csv(
    ///     "city,pop\nBrisbane,100000\nSydney,200000\n".as_bytes()
    /// ).unwrap();
    ///
    /// // default display
    /// let x = t.clone().display().to_string();
    /// assert_eq!(&x, "\
    /// +----------+--------+
    /// | city     | pop    |
    /// |----------+--------|
    /// | Brisbane | 100000 |
    /// |----------+--------|
    /// | Sydney   | 200000 |
    /// +----------+--------+");
    ///
    /// // change the style
    /// let x = t.display()
    ///     .load_preset(deps::comfy_table::presets::ASCII_MARKDOWN)
    ///     .to_string();
    /// assert_eq!(&x, "\
    /// | city     | pop    |
    /// | Brisbane | 100000 |
    /// | Sydney   | 200000 |");
    /// ```
    pub fn display(self) -> comfy_table::Table
    where
        T: fmt::Display,
    {
        let Self { cells, rows, cols } = self;

        let mut t = comfy_table::Table::new();
        t.add_row(cols);

        for r in rows {
            t.add_row(r.into_iter().map(|i| &cells[i]));
        }

        t
    }

    fn maybe_consolidate(self) -> Self {
        let threshold = 5 * self.rows.len() * self.cols.len();
        if self.cells.len() <= threshold {
            self
        } else {
            self.consolidate()
        }
    }

    /// Reduce the backing cells data to only keep data that is currently in the table.
    ///
    /// It is generally not necessary to call this.
    pub fn consolidate(mut self) -> Self {
        // collect the indices that are in use
        let used_idxs = self
            .rows
            .iter()
            .flatten()
            .copied()
            .collect::<HashSet<CellKey>>();

        let mut i = 0; // the cell index
        let mut kept = 0; // the new index
        let mut remap = HashMap::new(); // a map of the old index to the new one
        self.cells.retain(|_| {
            // we retain if the index is in use
            let x = used_idxs.contains(&i);
            if x {
                remap.insert(i, kept); // i will map to kept
                kept += 1; // the kept index will be incremented
            }

            i += 1; // always increment the cell index

            x
        });

        // now all the indices in the rows have to be remapped
        for row in &mut self.rows {
            for c in row {
                *c = *remap.get(c).expect("all indices should be remapped");
            }
        }

        self
    }
}

impl Table<String> {
    /// Read a table from CSV data, reading in as strings.
    ///
    /// # Example
    /// ```rust
    /// use rust_script_ext::prelude::*;
    /// let t = Table::from_csv(
    ///     "city,pop\nBrisbane,100000\nSydney,200000\n".as_bytes()
    /// ).unwrap();
    /// let x = t.display().to_string();
    /// assert_eq!(&x, "\
    /// +----------+--------+
    /// | city     | pop    |
    /// |----------+--------|
    /// | Brisbane | 100000 |
    /// |----------+--------|
    /// | Sydney   | 200000 |
    /// +----------+--------+");
    /// ```
    pub fn from_csv<R: Read>(rdr: R) -> Result<Self> {
        let mut csv = csv::Reader::from_reader(rdr);
        let cols = csv
            .headers()
            .into_diagnostic()?
            .into_iter()
            .map(|x| x.to_string())
            .collect();

        let mut rows = Vec::new();
        let mut cells = Vec::new();

        for row in csv.records() {
            let row = row.into_diagnostic()?;
            let mut r = Vec::with_capacity(row.len());
            for cell in row.into_iter() {
                r.push(cells.len());
                cells.push(cell.to_string());
            }
            rows.push(r);
        }

        Ok(Self { cols, rows, cells })
    }
}

fn build_header(cols: &[String]) -> Header {
    cols.iter()
        .enumerate()
        .map(|(i, h)| (h.as_str(), i))
        .collect()
}

impl<T: AsRef<[u8]>> WriteAs<CSV, ()> for Table<T> {
    fn write_as(&self, _: CSV, wtr: &mut dyn std::io::Write) -> Result<()> {
        let mut csv = csv::Writer::from_writer(wtr);
        csv.write_record(&self.cols).into_diagnostic()?;

        for row in &self.rows {
            csv.write_record(row.into_iter().map(|&i| &self.cells[i]))
                .into_diagnostic()?;
        }

        Ok(())
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

impl<C: Column + ?Sized> Column for &C {
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
