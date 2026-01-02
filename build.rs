extern crate winres;

fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("src/icons/tray/main.ico");

    // Application manifest for dark mode support
    res.set_manifest(r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0" xmlns:asmv3="urn:schemas-microsoft-com:asm.v3">
    <asmv3:application>
        <asmv3:windowsSettings>
            <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true</dpiAware>
            <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
            <activeCodePage xmlns="http://schemas.microsoft.com/SMI/2019/WindowsSettings">UTF-8</activeCodePage>
        </asmv3:windowsSettings>
    </asmv3:application>
    <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
        <application>
            <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
        </application>
    </compatibility>
    <dependency>
        <dependentAssembly>
            <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
        </dependentAssembly>
    </dependency>
</assembly>
"#);

    // register light mode icons (10,20,...,50)
    for i in (10..=50).step_by(10) {
        res.set_icon_with_id(&format!("src/icons/tray/battery{i}.ico"), &format!("{i}"));
        let charging_i = i + 1;
        res.set_icon_with_id(
            &format!("src/icons/tray/battery{charging_i}.ico"),
            &format!("{charging_i}"),
        );
    }

    for i in (15..=55).step_by(10) {
        res.set_icon_with_id(&format!("src/icons/tray/battery{i}.ico"), &format!("{i}"));
        let charging_i = i + 1;
        res.set_icon_with_id(
            &format!("src/icons/tray/battery{charging_i}.ico"),
            &format!("{charging_i}"),
        );
    }

    res.compile().unwrap();
}
