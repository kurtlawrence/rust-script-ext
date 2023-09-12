use crate::prelude::{Deserialize, IntoDiagnostic, Result, Serialize, WrapErr};
use std::{
    borrow::Borrow,
    io::{Read, Write},
};

/// Defines a _structured_ format which can be used with [`ReadAs`]/[`WriteAs`].
///
/// A format needs to describe (de)serialisation mechanism, along with the input/output types.
/// Note the use of GATs, this can be leveraged to work with containerised types.
pub trait Format {
    /// The resulting type after _deserialisation_.
    type Output<T>;
    /// The input for _serialisation_.
    ///
    /// Note that this is passed through as a reference to `serialise`.
    type Input<T>: ?Sized;

    /// Deserialise from `rdr` into `Output`.
    fn deserialise<T>(rdr: &mut dyn Read) -> Result<Self::Output<T>>
    where
        for<'de> T: Deserialize<'de>;

    /// Serialise `val` into `wtr`.
    fn serialise<T>(wtr: &mut dyn Write, val: &Self::Input<T>) -> Result<()>
    where
        T: Serialize;
}

/// A trait which gives any [`Read`]er the `read_as` function which can be used to read with a
/// specific format.
///
/// Noteable examples of using `read_as` would be to read a file directly as CSV/json/toml.
///
/// # Example
/// ```rust
/// # use rust_script_ext::prelude::*;
/// #[derive(Deserialize, Debug, PartialEq)]
/// struct City {
///     city: String,
///     pop: u32,
/// }
///
/// let csv = "city,pop\nBrisbane,100000\nSydney,200000\n";
///
/// // read_as on anything that is Read
/// let x = csv.as_bytes().read_as::<CSV, City>().unwrap();
///
/// assert_eq!(
///     x,
///     vec![
///         City {
///             city: "Brisbane".to_string(),
///             pop: 100_000,
///         },
///         City {
///             city: "Sydney".to_string(),
///             pop: 200_000,
///         }
///     ]
/// );
/// ```
pub trait ReadAs {
    /// Read the data as if it is structured with format `F`, deserialising into `F::Output<T>`.
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

/// A trait which can supply `write_as` on a type to serialise it into a specific format and write it into a
/// [`Write`]r.
///
/// Its counterpart [`ReadAs`] works on any reader, `WriteAs` works differently, being available on
/// any _type_ which matches the _format's input_.
///
/// # Example
/// ```rust
/// # use rust_script_ext::prelude::*;
/// #[derive(Serialize)]
/// struct City {
///     city: String,
///     pop: u32,
/// }
///
/// let mut buf = Vec::new();
///
/// let sydney = City {
///     city: "Sydney".to_string(),
///     pop: 200_000
/// };
///
/// // we serialise as JSON
/// sydney.write_as(JSON, &mut buf).unwrap();
///
/// assert_eq!(buf, r#"{
///   "city": "Sydney",
///   "pop": 200000
/// }"#.as_bytes());
///
/// // but we could also easily serialise as TOML
/// buf.clear();
/// sydney.write_as(TOML, &mut buf).unwrap();
///
/// assert_eq!(buf, r#"city = "Sydney"
/// pop = 200000
/// "#.as_bytes());
/// ```
pub trait WriteAs<F, T> {
    /// Serialise this with the format `F` into the `wtr`.
    fn write_as(&self, fmt: F, wtr: &mut dyn Write) -> Result<()>;
}

impl<F, T, A> WriteAs<F, T> for A
where
    F: Format,
    T: Serialize,
    A: Borrow<F::Input<T>>,
{
    fn write_as(&self, _: F, wtr: &mut dyn Write) -> Result<()> {
        F::serialise(wtr, self.borrow())
    }
}

/// A CSV [`Format`].
///
/// - The _output_ is `Vec<T>` (`T: Deserialize`).
/// - The _input_ is `[T]` (`T: Serialize`).
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

    fn serialise<T>(wtr: &mut dyn Write, val: &[T]) -> Result<()>
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

/// A json [`Format`].
///
/// - The _output_ is `T` (`T: Deserialize`).
/// - The _input_ is `T` (`T: Serialize`).
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
        serde_json::to_writer_pretty(wtr, &val)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to serialise {} as JSON", std::any::type_name::<T>()))
    }
}

/// A toml [`Format`].
///
/// - The _output_ is `T` (`T: Deserialize`).
/// - The _input_ is `T` (`T: Serialize`).
pub struct TOML;
impl Format for TOML {
    type Output<T> = T;
    type Input<T> = T;

    fn deserialise<T>(rdr: &mut dyn Read) -> Result<Self::Output<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        let mut s = String::new();
        rdr.read_to_string(&mut s)
            .into_diagnostic()
            .wrap_err("failed reading TOML data to string")?;

        toml::from_str(&s).into_diagnostic().wrap_err_with(|| {
            format!(
                "failed to deserialise {} from TOML",
                std::any::type_name::<T>()
            )
        })
    }

    fn serialise<T>(wtr: &mut dyn Write, val: &T) -> Result<()>
    where
        T: Serialize,
    {
        let s = toml::to_string_pretty(val)
            .into_diagnostic()
            .wrap_err_with(|| {
                format!("failed to serialise {} as JSON", std::any::type_name::<T>())
            })?;

        wtr.write_all(s.as_bytes())
            .into_diagnostic()
            .wrap_err("failed to write TOML to writer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct City {
        city: String,
        pop: u32,
    }

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

        // check that vec can work without as_slice
        let mut buf = Vec::new();
        x.write_as(CSV, &mut buf).unwrap();

        assert_eq!(buf, csv.as_bytes());
    }

    #[test]
    fn structured_api_json() {
        let data = serde_json::json!({
            "city": "Brisbane",
            "pop": 100_000
        });

        let x = data.to_string().as_bytes().read_as::<JSON, City>().unwrap();

        assert_eq!(
            x,
            City {
                city: "Brisbane".to_string(),
                pop: 100_000,
            }
        );

        let mut buf = Vec::new();
        x.write_as(JSON, &mut buf).unwrap();

        assert_eq!(
            buf,
            r#"{
  "city": "Brisbane",
  "pop": 100000
}"#
            .as_bytes()
        );
    }
}
