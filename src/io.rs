use crate::prelude::{Serialize, Deserialize, Result, IntoDiagnostic, WrapErr};
use std::io::{Read, Write};

pub trait Format {
    type Output<T>;
    type Input<T>: ?Sized;

    fn deserialise<T>(rdr: &mut dyn Read) -> Result<Self::Output<T>>
    where
        for<'de> T: Deserialize<'de>;

    fn serialise<T>(wtr: &mut dyn Write, val: &Self::Input<T>) -> Result<()>
    where
        T: Serialize;
}

pub trait ReadAs {
    fn read_as<F, T>(&mut self) -> Result<F::Output<T>>
    where
        F: Format,
        for<'de> T: Deserialize<'de>;
}

impl<R: Read> ReadAs for R {
    fn read_as<F, T>(&mut self) -> Result<F::Output<T>>
    where
        F: Format,
        for<'de> T: Deserialize<'de>,
    {
        F::deserialise(self)
    }
}

pub trait WriteAs<F, T> {
    fn write_as(&self, fmt: F, wtr: &mut dyn Write) -> Result<()>;
}

impl<F, T> WriteAs<F, T> for F::Input<T>
where
    F: Format,
    T: Serialize,
{
    fn write_as(&self, _: F, wtr: &mut dyn Write) -> Result<()> {
        F::serialise(wtr, self)
    }
}

pub struct CSV;
impl Format for CSV {
    type Output<T> = Vec<T>;
    type Input<T> = [T];

    fn deserialise<T>(rdr: &mut dyn Read) -> Result<Self::Output<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        let mut v = Vec::new();
        for r in ::csv::Reader::from_reader(rdr).into_deserialize() {
            let r: T = r.into_diagnostic()?;
            v.push(r);
        }

        Ok(v)
    }

    fn serialise<'a, T>(wtr: &mut dyn Write, val: &'a [T]) -> Result<()>
    where
        T: Serialize,
    {
        let mut csv = ::csv::Writer::from_writer(wtr);
        for x in val {
            csv.serialize(x).into_diagnostic()?;
        }

        Ok(())
    }
}

pub struct JSON;
impl Format for JSON {
    type Output<T> = T;
    type Input<T> = T;

    fn deserialise<T>(rdr: &mut dyn Read) -> Result<Self::Output<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        serde_json::from_reader(rdr)
            .into_diagnostic()
            .wrap_err_with(|| {
                format!(
                    "failed to deserialise {} from JSON",
                    std::any::type_name::<T>()
                )
            })
    }

    fn serialise<T>(wtr: &mut dyn Write, val: &T) -> Result<()>
    where
        T: Serialize,
    {
        serde_json::to_writer(wtr, &val)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to serialise {} as JSON", std::any::type_name::<T>()))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_api_csv() {
        let csv = "city,pop\nBrisbane,100000\nSydney,200000\n";

        let x = csv.as_bytes().read_as::<CSV, City>().unwrap();

        assert_eq!(
            x,
            vec![
                City {
                    city: "Brisbane".to_string(),
                    pop: 100_000,
                },
                City {
                    city: "Sydney".to_string(),
                    pop: 200_000,
                }
            ]
        );

        let mut buf = Vec::new();
        x.as_slice().write_as(CSV, &mut buf).unwrap();

        assert_eq!(buf, csv.as_bytes());
    }

    #[test]
    fn structured_api_json() {
        let data = serde_json::json!({
            "city": "Brisbane",
            "pop": 100_000
        })
        .to_string();

        let x = data.as_bytes().read_as::<JSON, City>().unwrap();

        assert_eq!(
            x,
            City {
                city: "Brisbane".to_string(),
                pop: 100_000,
            }
        );

        let mut buf = Vec::new();
        x.write_as(JSON, &mut buf).unwrap();

        assert_eq!(buf, data.as_bytes());
    }
}
