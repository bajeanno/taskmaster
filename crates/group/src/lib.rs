use libc::{sys::types::Gid, unistd::GroupRaw};
use std::{
    ffi::{CStr, CString},
    io,
    mem::size_of,
    process::Command,
    vec,
};

#[derive(Debug, thiserror::Error)]
pub enum GroupError {
    #[error("Failed to check if group already exists:")]
    GroupCheck {
        #[source]
        error: std::io::Error,
        group_name: String,
    },
    #[error("Failed to create taskmaster group on the system")]
    GroupCreate {
        #[source]
        error: std::io::Error,
        group_name: String,
    },
}

#[allow(unused)]
pub struct Group {
    name: String,
    password: String,
    pub gid: Gid,
    members: Vec<String>,
}

// The caller must ensure `raw` points to a valid C `struct group`
// and remains valid while this function copies its contents.
unsafe fn raw_to_group(raw: *const GroupRaw) -> Option<Group> {
    let raw = unsafe { raw.as_ref()? };

    if raw.gr_name.is_null() || raw.gr_passwd.is_null() || raw.gr_mem.is_null() {
        return None;
    }

    let name: String =
        String::from_utf8_lossy_owned(unsafe { CStr::from_ptr(raw.gr_name) }.to_bytes().to_vec());
    let password: String =
        String::from_utf8_lossy_owned(unsafe { CStr::from_ptr(raw.gr_passwd) }.to_bytes().to_vec());

    let mut members = Vec::new();
    let mut offset = 0;

    loop {
        let member = unsafe {
            std::ptr::read_unaligned(
                (raw.gr_mem as *const u8)
                    .add(offset)
                    .cast::<*mut std::ffi::c_char>(),
            )
        };
        if member.is_null() {
            break;
        }
        members.push(String::from_utf8_lossy_owned(
            unsafe { CStr::from_ptr(member) }.to_bytes().to_vec(),
        ));
        offset += size_of::<*mut std::ffi::c_char>();
    }

    Some(Group {
        name,
        password,
        gid: raw.gr_gid,
        members,
    })
}

impl Group {
    pub fn from_name(group_name: &str) -> Result<Option<Group>, GroupError> {
        let name = CString::new(group_name).expect("The group name provided contains a 0 byte");
        let mut group = std::mem::MaybeUninit::<GroupRaw>::uninit();
        let mut buffer = vec![0u8; 16 * 1024];
        let mut result: *mut GroupRaw = std::ptr::null_mut();
        let result_code = unsafe {
            libc::unistd::getgrnam_r(
                name.as_ptr(),
                group.as_mut_ptr(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &mut result,
            )
        };

        if result_code != 0 {
            return Err(io::Error::from_raw_os_error(result_code)).map_err(|err| {
                GroupError::GroupCheck {
                    error: err,
                    group_name: group_name.to_owned(),
                }
            });
        }

        if result.is_null() {
            return Ok(None);
        }

        Ok(unsafe { raw_to_group(result) })
    }
}

#[cfg(target_os = "macos")]
fn create_group_creation_command_for_os(name: &str) -> Command {
    let mut command = Command::new("/usr/sbin/dseditgroup");
    command.args(["-o", "create", "-n", ".", name]);
    command
}

#[cfg(not(target_os = "macos"))]
fn create_group_creation_command_for_os(name: &str) -> Command {
    let mut command = Command::new("groupadd");
    command.arg(name);
    command
}

pub fn create_group(name: &'static str) -> Result<u32, GroupError> {
    create_group_creation_command_for_os(name)
        .spawn()
        .map_err(|err| GroupError::GroupCreate {
            error: err,
            group_name: name.to_owned(),
        })?
        .wait()
        .map_err(|err| GroupError::GroupCreate {
            error: err,
            group_name: name.to_owned(),
        })?;
    Ok(Group::from_name(name)?
        .expect("group has just been set and should be registered on the system")
        .gid)
}

pub fn is_in_group(name: &str) -> Result<bool, GroupError> {
    let res: i32 = unsafe { libc::unistd::getgroups(0, std::ptr::null_mut()) };
    if res < 0 {
        return Err(std::io::Error::from_raw_os_error(res)).map_err(|err| GroupError::GroupCheck {
            error: err,
            group_name: name.to_owned(),
        });
    }
    let count = res as usize;
    let mut groups = vec![0; count];
    let res: i32 = unsafe { libc::unistd::getgroups(groups.len() as i32, groups.as_mut_ptr()) };
    if res < 0 {
        return Err(std::io::Error::from_raw_os_error(res)).map_err(|err| GroupError::GroupCheck {
            error: err,
            group_name: name.to_owned(),
        });
    }

    match Group::from_name(name)? {
        Some(group) => Ok(groups.contains(&group.gid)),
        None => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert!(is_in_group("taskmaster").is_ok())
    }

    #[test]
    fn admin_group() {
        assert!(
            Group::from_name("admin").unwrap().is_some(),
            "group admin should exist"
        );
    }

    #[test]
    fn everyone_group() {
        assert!(
            Group::from_name("everyone").unwrap().is_some(),
            "group everyone should exist"
        );
    }

    #[test]
    fn wheel_group() {
        assert!(
            Group::from_name("wheel").unwrap().is_some(),
            "group wheel should exist"
        );
    }

    #[test]
    fn taskmaster_group() {
        assert!(
            Group::from_name("taskmaster").unwrap().is_some(),
            "group taskmaster should exist"
        );
    }
}
