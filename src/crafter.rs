use std::path::PathBuf;

pub struct CrafterEntry<I, O>
where
    I: Iterator<Item = PathBuf>,
    O: Iterator<Item = (PathBuf, Vec<u8>)>,
{
    input_relpaths: I,
    output_relpath_bins: O,
}

pub struct ListEntry {
    path: PathBuf,
    is_file: bool,
}
pub trait ObjectDatabaseRead {
    fn read(&self, path: &PathBuf) -> Vec<u8>;
    fn list(&self, path: &PathBuf) -> impl Iterator<Item = ListEntry>;
}
pub trait ObjectDatabaseWrite: ObjectDatabaseRead {
    fn write(&self, path: &PathBuf, data: &[u8]);
}
pub trait Crafter {
    fn flatten(save_dir: impl ObjectDatabaseRead, odb: impl ObjectDatabaseWrite);
    fn unflatten(save_dir: impl ObjectDatabaseWrite, odb: impl ObjectDatabaseRead);
}

struct RawFlattenCrafter;
impl Crafter for RawFlattenCrafter {
    fn flatten(save: impl ObjectDatabaseRead, odb: impl ObjectDatabaseWrite) {
        // save.list(&PathBuf::from("."))
        //     .find(|e| e.name == "icon.png" && e.is_file);
        todo!()
    }

    fn unflatten(save: impl ObjectDatabaseWrite, odb: impl ObjectDatabaseRead) {
        todo!()
    }
}
