# Headset Battery Indicator

Adds a small icon to the Windows task bar, displaying the battery level of most* connected wireless headphones.

![Screenshot of indicator on Windows task bar](docs/icon-screenshot.png)

## Features

* Works on Windows 10+
* Built using Rust, with low resource usage (~2MB RAM)
* Supports most non-bluetooth headsets (SteelSeries, Logitech, Corsair, HyperX)
  * See all [supported headsets here](https://github.com/Sapd/HeadsetControl?tab=readme-ov-file#supported-headsets).
  
    > Since headset support is done by reverse-engineering the USB HID protocol, not every headset is supported yet, and some headsets (notably Arctis Wireless 1) may not work even though they are listed as supported.

* Shows a little green dot to indicate charging

  ![Charging icon](docs/icon-charging.png)

* You can also choose to show a percentage number instead of an icon

  ![Percentage icon](docs/icon-text.png)

* Shows notifications on low battery level or when finished charging (optional)

Headset Battery Indicator depends on [HeadsetControl](https://github.com/Sapd/HeadsetControl), which is licensed under GPL v3.

## Installation

* Download the [latest release](https://github.com/aarol/headset-battery-indicator/releases/latest) and run the installer

> Running the installer may result in a Windows defender SmartScreen warning. This happens to all executables that don't have a large enough install count. There's no way around it other than paying hundreds of dollars every year for a signed certificate from Microsoft :(
>
>Additionally, sometimes Windows Defender's ML-based antivirus falsely detects the program as a virus, most commonly `Wacatac.b!ml` (the "ml" at the end signifies machine learning).

## Security

The code that is in this repository is the code that is in the executable. There is a [Github Action](https://github.com/aarol/headset-battery-indicator/actions) that builds the code from source and creates the release in the [releases page](https://github.com/aarol/headset-battery-indicator/releases).

The GitHub release is marked as immutable, so once the executable is built by the Actions workflow, it cannot be modified by me or anyone else. This way, it is guaranteed that the code you're running is the same code that is in this repository.

## Troubleshooting

If you're experiencing crashes or other issues, you can try running the `headset-battery-indicator-debug.exe` located at `%localAppData%\Programs\HeadsetBatteryIndicator` or look at the log file located in the same folder.

### Why does it only show 100%, 75%, 50%, 25% or 0%?

This is limitation of the headsets themselves, as some devices only expose 5 possible battery states.

### My headset is connected, but it still shows "No headphone adapter found"

Your headset might be unsupported due to being a new model. See [Adding a new headset](#adding-a-new-headset)

## Development

Git, Rust and Cargo need to be installed.

Additionally, this project depends on [hidapi](https://github.com/libusb/hidapi) and [HeadsetControl](https://github.com/Sapd/HeadsetControl) which are stored as Git submodules in the [vendor](./vendor/) directory. Those C/C++ projects are built in [build.rs](./build.rs) using the `cc` crate.
 
The MSVC compiler is also required. Install Visual Studio 2019 or later with "Desktop development with C++","Windows SDK" and "MSVC .. C++ x64/x86 build tools" workloads.

To get started with developing the project, first clone this repository:

```sh
git clone https://github.com/aarol/headset-battery-indicator --recursive
```

Remember to use `--recursive` to clone the submodules as well.

Then, from the `headset-battery-indicator` folder, you can:

* Run the application in release mode: `cargo run --release`

* Run the application in debug mode: `cargo run`

* Debug the application by pressing `F5` in VS Code with the Rust Analyzer and CodeLLDB extensions installed.

* Build the installer: install [Inno Setup Compiler](https://jrsoftware.org/isinfo.php), open [installer.iss](installer.iss) and click "Compile".

## Translations

There are translations for the following languages:

* English
* Finnish
* Italian
* German
* Portuguese
* Spanish

Translations can be added to the [lang.rs](./src/lang.rs) file.

## Adding a new headset

Headset Battery Indicator depends on [HeadsetControl](https://github.com/Sapd/HeadsetControl) for supporting many kinds of headset models. If the headset you're using isn't currently supported, you can either wait until someone else adds support for it, or try adding it yourself.

Reading the [HeadsetControl wiki](https://github.com/Sapd/HeadsetControl/wiki/Development-1-%E2%80%90-Adding-a-device) is the best resource on this.

I have a post on my website going a bit into reverse-engineering the headset APIs as well: <https://aarol.dev/posts/arctis-hid>

### Arctis headphone firmware update

In early 2026, many SteelSeries Arctis headphones received a new firmware update, adding more accurate battery level reporting, but also changing the HID product ID, leading to HeadsetControl (and thus this program) not recognizing the headset.

Luckily, the fix is quite simple. The new product ID simply needs to be reported to the maintainers.

Here's how you can find the product ID:

1. Open "Device Manager" on Windows
2. Find your headset in the list and open "**properties**"
3. In the Details tab, select the "device instance path" property. It will contain the vendor ID (VID) and product ID (PID).

<details><summary>Reference screenshot</summary>
 <img width="400" height="455" alt="image" src="https://github.com/user-attachments/assets/47a21657-cc0f-4ed1-bc8c-7d8e8ece8827" />
</details>

Once you have the vID and pID, you can file an issue in the repository or in [HeadsetControl](https://github.com/Sapd/HeadsetControl), or file a [pull request](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/proposing-changes-to-your-work-with-pull-requests/creating-a-pull-request) if you're more technically savvy!

### License

This project is licensed under GNU GPL v3.

You’re free to use, modify, and redistribute it, as long as your version is also licensed under GPL v3, and you include the source code and license when you share it.
See the [LICENSE](./LICENSE) file for full terms.
