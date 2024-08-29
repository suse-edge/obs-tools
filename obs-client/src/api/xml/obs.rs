#[derive(Debug, Clone, yaserde::YaDeserialize, PartialEq, Eq, Hash)]
pub enum PackageCode {
    #[yaserde(rename = "unresolvable")]
    Unresolvable,
    #[yaserde(rename = "succeeded")]
    Succeeded,
    #[yaserde(rename = "failed")]
    Failed,
    #[yaserde(rename = "broken")]
    Broken,
    #[yaserde(rename = "disabled")]
    Disabled,
    #[yaserde(rename = "excluded")]
    Excluded,
    #[yaserde(rename = "blocked")]
    Blocked,
    #[yaserde(rename = "locked")]
    Locked,
    #[yaserde(rename = "unknown")]
    Unknown,
    #[yaserde(rename = "scheduled")]
    Scheduled,
    #[yaserde(rename = "building")]
    Building,
    #[yaserde(rename = "finished")]
    Finished,
}

impl Default for PackageCode {
    fn default() -> Self {
        Self::Unknown
    }
}

impl PackageCode {
    pub fn is_ok(&self) -> bool {
        [
            PackageCode::Succeeded,
            PackageCode::Disabled,
            PackageCode::Excluded,
        ]
        .contains(self)
    }
}

#[derive(Debug, Clone, yaserde::YaDeserialize, PartialEq, Eq, Hash)]
pub enum RepositoryCode {
    #[yaserde(rename = "unknown")]
    Unknown,
    #[yaserde(rename = "broken")]
    Broken,
    #[yaserde(rename = "scheduling")]
    Scheduling,
    #[yaserde(rename = "blocked")]
    Blocked,
    #[yaserde(rename = "building")]
    Building,
    #[yaserde(rename = "finished")]
    Finished,
    #[yaserde(rename = "publishing")]
    Publishing,
    #[yaserde(rename = "published")]
    Published,
    #[yaserde(rename = "unpublished")]
    Unpublished,
}

impl Default for RepositoryCode {
    fn default() -> Self {
        Self::Unknown
    }
}

impl From<&str> for RepositoryCode {
    fn from(value: &str) -> Self {
        match value {
            "broken" => Self::Broken,
            "scheduling" => Self::Scheduling,
            "blocked" => Self::Blocked,
            "building" => Self::Building,
            "finished" => Self::Finished,
            "publishing" => Self::Publishing,
            "published" => Self::Published,
            "unpublished" => Self::Unpublished,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, yaserde::YaDeserialize, PartialEq, Eq, Hash)]
pub enum BuildArch {
    #[yaserde(rename = "noarch")]
    NoArch,

    #[yaserde(rename = "aarch64")]
    Aarch64,
    #[yaserde(rename = "aarch64_ilp32")]
    Aarch64Ilp32,

    #[yaserde(rename = "armv4l")]
    Armv4l,
    #[yaserde(rename = "armv5l")]
    Armv5l,
    #[yaserde(rename = "armv6l")]
    Armv6l,
    #[yaserde(rename = "armv7l")]
    Armv7l,
    #[yaserde(rename = "armv5el")]
    Armv5el,
    #[yaserde(rename = "armv6el")]
    Armv6el,
    #[yaserde(rename = "armv7el")]
    Armv7el,
    #[yaserde(rename = "armv8el")]
    Armv8el,

    #[yaserde(rename = "hppa")]
    Hppa,

    #[yaserde(rename = "m68k")]
    M68k,

    #[yaserde(rename = "i386")]
    I386,
    #[yaserde(rename = "i486")]
    I486,
    #[yaserde(rename = "i586")]
    I586,
    #[yaserde(rename = "i686")]
    I686,
    #[yaserde(rename = "athlon")]
    Athlon,

    #[yaserde(rename = "ia64")]
    Ia64,

    #[yaserde(rename = "k1om")]
    K1om,

    #[yaserde(rename = "loongarch64")]
    Loongarch64,

    #[yaserde(rename = "mips")]
    Mips,
    #[yaserde(rename = "mipsel")]
    Mipsel,
    #[yaserde(rename = "mips32")]
    Mips32,
    #[yaserde(rename = "mips64")]
    Mips64,
    #[yaserde(rename = "mips64el")]
    Mips64el,

    #[yaserde(rename = "ppc")]
    Ppc,
    #[yaserde(rename = "ppc64")]
    Ppc64,
    #[yaserde(rename = "ppc64p7")]
    Ppc64p7,
    #[yaserde(rename = "ppc64le")]
    Ppc64le,

    #[yaserde(rename = "riscv64")]
    Riscv64,

    #[yaserde(rename = "s390")]
    S390,
    #[yaserde(rename = "s390x")]
    S390x,

    #[yaserde(rename = "sh4")]
    Sh4,

    #[yaserde(rename = "sparc")]
    Sparc,
    #[yaserde(rename = "sparc64")]
    Sparc64,
    #[yaserde(rename = "sparc64v")]
    Sparc64v,
    #[yaserde(rename = "sparcv8")]
    Sparcv8,
    #[yaserde(rename = "sparcv9")]
    Sparcv9,
    #[yaserde(rename = "sparcv9v")]
    Sparcv9v,

    #[yaserde(rename = "x86_64")]
    X86_64,

    #[yaserde(rename = "local")]
    Local,
}

impl Default for BuildArch {
    fn default() -> Self {
        Self::NoArch
    }
}

impl std::fmt::Display for BuildArch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let variant = to_snake_case(&format!("{:?}", self));
        f.write_str(&variant)
    }
}

fn to_snake_case(s: &str) -> String {
    let mut snake = String::new();
    for (i, ch) in s.char_indices() {
        if i > 0 && ch.is_uppercase() {
            snake.push('_');
        }
        snake.push(ch.to_ascii_lowercase());
    }
    snake
}
