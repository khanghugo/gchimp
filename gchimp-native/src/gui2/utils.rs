static mut GLOBAL_ID: usize = 0;

struct GlobalId;

impl GlobalId {
    fn assign_id(&self) -> usize {
        let res = unsafe { GLOBAL_ID };

        unsafe { GLOBAL_ID += 1 };

        res
    }
}

pub const IMAGE_FORMATS: &[&str] = &["png", "bmp", "jpeg", "jpg"];
