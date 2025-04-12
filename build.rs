/*
 * Copyright (c):
 * 2025 zephyrj
 * zephyrj@protonmail.com
 *
 * This file is part of engine-crane.
 *
 * engine-crane is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * engine-crane is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with engine-crane. If not, see <https://www.gnu.org/licenses/>.
 */

#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn main() {
    if Ok("release".to_owned()) == std::env::var("PROFILE") {
        let mut res = winres::WindowsResource::new();
        res.set_manifest(manifest_data());
        res.set("RC_FILE", "res/app.rc");
        res.set_icon("src/ui/resources/eng-logo.ico");
        res.compile().unwrap();
    }
}

#[cfg(feature = "always_admin")]
fn manifest_data() -> &'static str{
    return r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
<trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
        <requestedPrivileges>
            <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
        </requestedPrivileges>
    </security>
</trustInfo>
</assembly>
"#;
}

#[cfg(not(feature = "always_admin"))]
fn manifest_data() -> &'static str {
    return r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
<trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
        <requestedPrivileges>
            <requestedExecutionLevel level="asInvoker" uiAccess="false" />
        </requestedPrivileges>
    </security>
</trustInfo>
</assembly>
"#;
}

#[cfg(unix)]
fn main() {
}
