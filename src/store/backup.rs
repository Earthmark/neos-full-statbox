use super::CowStr;
use chrono::{DateTime, Utc};
use core::panic;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::BTreeMap, ffi::OsStr, path::PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serde: {0} ({1})")]
    Serde(serde_json::Error, PathBuf),
}

fn os_to_cow(s: &OsStr) -> CowStr {
    s.to_string_lossy().into_owned().into()
}

trait FromDisk: Sized {
    fn from_disk(p: PathBuf) -> Result<Self, Error>;
}

impl<T: FromDisk> FromDisk for BTreeMap<CowStr, T> {
    fn from_disk(p: PathBuf) -> Result<Self, Error> {
        let dir = p.read_dir()?;
        let mut map = BTreeMap::<CowStr, T>::default();
        for dir in dir.into_iter() {
            let dir = dir?;
            let name = os_to_cow(&dir.path().file_stem().unwrap());
            let item = T::from_disk(dir.path())?;
            map.insert(name, item);
        }
        Ok(map)
    }
}

impl<T: FromDisk> FromDisk for Vec<T> {
    fn from_disk(p: PathBuf) -> Result<Self, Error> {
        let dir = p.read_dir()?;
        let mut vec = Vec::<T>::default();
        for dir in dir.into_iter() {
            let dir = dir?;
            let item = T::from_disk(dir.path())?;
            vec.push(item);
        }
        Ok(vec)
    }
}

trait FromFile: DeserializeOwned {}

impl<T> FromDisk for T
where
    T: FromFile,
{
    fn from_disk(p: PathBuf) -> Result<Self, Error> {
        from_file(p)
    }
}

fn from_file<T>(p: PathBuf) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    let content = std::fs::File::open(&p)?;
    let buf_content = std::io::BufReader::new(content);
    let result = serde_json::from_reader(buf_content).map_err(|e| Error::Serde(e, p))?;
    Ok(result)
}

impl Backup {
    pub fn load(root: PathBuf) -> Result<Self, Error> {
        Self::from_disk(root)
    }
}

impl FromDisk for Backup {
    fn from_disk(p: PathBuf) -> Result<Self, Error> {
        let mut backup = Self::default();

        for dir in p.read_dir()?.into_iter() {
            let dir = dir?;

            if dir.file_name() == "Assets" {
                backup.assets = BTreeMap::<CowStr, AssetHandle>::from_disk(dir.path())?;
            } else {
                let (name, acc) = Account::load(dir.path())?;
                backup.accounts.insert(name, acc);
            }
        }
        Ok(backup)
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub assets: BTreeMap<CowStr, AssetHandle>,
    pub accounts: BTreeMap<CowStr, Account>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AssetHandle {
    pub path: PathBuf,
}

impl FromDisk for AssetHandle {
    fn from_disk(path: PathBuf) -> Result<Self, Error> {
        Ok(AssetHandle { path })
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub contacts: BTreeMap<CowStr, Contact>,
    pub group_members: BTreeMap<CowStr, BTreeMap<CowStr, GroupMember>>,
    pub groups: BTreeMap<CowStr, Group>,
    pub messages: BTreeMap<CowStr, Vec<Message>>,
    pub records: BTreeMap<CowStr, Record>,
    pub variable_definitions: BTreeMap<CowStr, VariableDefinition>,
    pub variables: BTreeMap<CowStr, String>,
}

impl Account {
    fn load(root: PathBuf) -> Result<(CowStr, Self), Error> {
        let name = os_to_cow(root.file_name().unwrap());
        let mut acc = Self::default();
        for dir in root.read_dir()?.into_iter() {
            let dir = dir?;
            match dir.file_name().to_str().unwrap() {
                "Contacts" => acc.contacts = BTreeMap::<CowStr, Contact>::from_disk(dir.path())?,
                "GroupMembers" => {}
                "Groups" => {}
                "Messages" => acc.messages = BTreeMap::<CowStr, Vec<Message>>::from_disk(dir.path())?,
                "Records" => acc.records = BTreeMap::<CowStr, Record>::from_disk(dir.path())?,
                "VariableDefinitions" => {}
                "Variables" => {}
                _ => panic!("Unknown folder in backup area!"),
            }
        }
        Ok((name, acc))
    }
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    id: CowStr,
    owner_id: CowStr,
    friend_username: CowStr,
    alternate_usernames: Option<CowStr>,
    friend_status: CowStr,
    is_accepted: bool,
    user_status: ContactStatus,
    #[serde(deserialize_with = "super::de::err_to_none")]
    latest_message_time: Option<DateTime<Utc>>,
    profile: Option<Profile>,
}

impl FromFile for Contact {}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContactStatus {
    online_status: CowStr,
    #[serde(deserialize_with = "super::de::err_to_none")]
    last_status_change: Option<DateTime<Utc>>,
    current_session_id: Option<CowStr>,
    current_session_access_level: i32,
    current_session_hidden: bool,
    current_hosting: bool,
    compatibility_hash: Option<CowStr>,
    neos_version: Option<CowStr>,
    #[serde(rename = "publicRSAKey")]
    public_rsa_key: Option<RsaKey>,
    output_device: CowStr,
    is_mobile: bool,
    #[serde(rename = "CurrentSession")]
    current_session: Option<Session>,
    active_sessions: Option<Vec<Session>>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct RsaKey {
    pub exponent: CowStr,
    pub modulus: CowStr,
    pub p: Option<CowStr>,
    pub q: Option<CowStr>,
    #[serde(rename = "DP")]
    pub dp: Option<CowStr>,
    #[serde(rename = "DQ")]
    pub dq: Option<CowStr>,
    pub inverse_q: Option<CowStr>,
    pub d: Option<CowStr>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub name: CowStr,
    pub description: Option<CowStr>,
    pub corresponding_world_id: Option<CorrespondingWorldId>,
    pub tags: Vec<CowStr>,
    pub session_id: CowStr,
    pub normalized_session_id: CowStr,
    pub host_user_id: CowStr,
    pub host_machine_id: CowStr,
    pub host_username: CowStr,
    pub compatibility_hash: CowStr,
    pub universe_id: Option<CowStr>,
    pub neos_version: CowStr,
    pub headless_host: bool,
    #[serde(rename = "sessionURLs")]
    pub session_urls: Vec<CowStr>,
    #[serde(deserialize_with = "super::de::null_to_default")]
    pub parent_session_ids: Vec<CowStr>,
    #[serde(deserialize_with = "super::de::null_to_default")]
    pub nested_session_ids: Vec<CowStr>,
    pub session_users: Vec<SessionUsers>,
    pub thumbnail: CowStr,
    pub joined_users: i32,
    pub active_users: i32,
    pub total_joined_users: i32,
    pub total_active_users: i32,
    pub max_users: i32,
    pub mobile_friendly: bool,
    pub session_begin_time: DateTime<Utc>,
    pub last_update: DateTime<Utc>,
    pub away_since: Option<DateTime<Utc>>,
    pub access_level: CowStr,
    #[serde(rename = "HasEnded")]
    pub has_ended: bool,
    #[serde(rename = "IsValid")]
    pub is_valid: bool,
    // There are more :D
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct CorrespondingWorldId {
    record_id: CowStr,
    owner_id: CowStr,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionUsers {
    pub username: CowStr,
    #[serde(rename = "userID")]
    pub user_id: CowStr,
    pub is_present: bool,
    pub output_device: i32,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    icon_url: CowStr,
    background_url: Option<CowStr>,
    tagline: Option<CowStr>,
    description: Option<CowStr>,
    profile_world_url: Option<CowStr>,
    #[serde(deserialize_with = "super::de::null_to_default")]
    showcase_items: Vec<CowStr>,
    #[serde(deserialize_with = "super::de::null_to_default")]
    token_opt_out: Vec<CowStr>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: CowStr,
    pub owner_id: CowStr,
    pub recipient_id: CowStr,
    pub message_type: MessageType,
    pub content: CowStr,
    pub send_time: DateTime<Utc>,
    pub last_update_time: DateTime<Utc>,
    pub read_time: Option<DateTime<Utc>>,
}

impl FromFile for Message {}

#[derive(Serialize, Deserialize, Debug, Default)]
pub enum MessageType {
    #[default]
    Object,
    Text,
    SessionInvite,
    Sound,
    CreditTransfer,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct VariableDefinition {
    pub definition_owner_id: CowStr,
    pub subpath: CowStr,
    pub variable_type: CowStr,
    pub default_value: CowStr,
    pub read_permissions: Vec<CowStr>,
    pub write_permissions: Vec<CowStr>,
    pub list_permissions: Vec<CowStr>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub owner_id: CowStr,
    pub path: CowStr,
    pub value: CowStr,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum RecordType {
    Audio,
    Directory,
    Link,
    #[default]
    Object,
    Texture,
    World,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: CowStr,
    pub owner_id: CowStr,
    pub asset_uri: Option<CowStr>, // Directory has null
    pub global_version: i32,
    pub local_version: i32,
    pub last_modifying_user_id: CowStr,
    pub last_modifying_machine_id: Option<CowStr>,
    pub name: CowStr,
    pub description: Option<CowStr>, // this is never populated?
    pub record_type: RecordType,
    pub owner_name: CowStr,
    #[serde(deserialize_with = "super::de::null_to_default")]
    pub tags: Vec<CowStr>,
    pub path: Option<CowStr>,
    pub thumbnail_uri: Option<CowStr>,
    #[serde(deserialize_with = "super::de::err_to_none")]
    pub last_modification_time: Option<DateTime<Utc>>,
    pub creation_time: Option<DateTime<Utc>>,
    pub first_publish_time: Option<DateTime<Utc>>,
    pub is_public: bool,
    pub is_for_patrons: bool,
    pub visits: i32,
    pub rating: i32,
    pub random_order: i32,
    #[serde(deserialize_with = "super::de::null_to_default")]
    pub submissions: Vec<Submission>,
    #[serde(deserialize_with = "super::de::null_to_default")]
    #[serde(rename = "neosDBmanifest")]
    pub neos_db_manifest: Vec<AssetRef>,
}

impl FromFile for Record {}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AssetRef {
    pub hash: CowStr,
    pub bytes: u64,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: CowStr,
    pub admin_user_id: CowStr,
    pub name: CowStr,
    pub quota_bytes: u64,
    pub used_bytes: u64,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Submission {
    pub id: CowStr,
    pub owner_id: CowStr,
    pub target_record_id: RecordId,
    pub submission_time: DateTime<Utc>,
    pub submitted_by_id: CowStr,
    pub submitted_by_name: CowStr,
    pub featured: bool,
    pub featured_by_user_id: Option<CowStr>,
    pub featured_timestamp: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecordId {
    pub record_id: CowStr,
    pub owner_id: CowStr,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GroupMember {
    id: CowStr,
    owner_id: CowStr,
    quota_bytes: u64,
    used_bytes: u64,
}
