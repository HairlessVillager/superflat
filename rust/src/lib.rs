use pyo3::prelude::*;

mod pumpkin;

#[pymodule]
fn _superflat(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let pumpkin_pymodule = PyModule::new(py, "pumpkin")?;
    pumpkin::init_submodule(&pumpkin_pymodule)?;

    m.add_submodule(&pumpkin_pymodule)?;

    let sys = py.import("sys")?;
    sys.getattr("modules")?
        .set_item("_superflat.pumpkin", &pumpkin_pymodule)?;
    Ok(())
}
