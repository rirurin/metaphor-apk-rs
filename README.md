# metaphor-apk-rs

A small Rust library for reading and writing APK files, a container format ([not that one!](https://en.wikipedia.org/wiki/Apk_(file_format)))
used for storing textures, usually one per file, in [Metaphor: ReFantazio](https://store.steampowered.com/app/2679460/Metaphor_ReFantazio/).
This library contains utilities for reading (`ApkReader`) and writing (`ApkWriter`) APK files.

This repository also contains a tool `metaphor-apk-pack` for extracing and repacking APK files, which is a Rust reimplementation of **DeathChaos'**
[MetaphorAPKPack](https://github.com/DeathChaos25/MetaphorAPKPack), with support for more compression formats.

*Note that to use APKs that are zstd compressed in game requires your mod to have [OpenGFD](https://github.com/rirurin/opengfd) set as a dependency*

## Library Examples

### Reading all files from an APK file

```rust
let mut apk = ApkReader::read("path/to/archive.apk")?;
for (name, bytes) in apk.get_all_files()? {
    std::fs::write(Path::new("/path/to/output").join(name), bytes.as_slice())?;
}
```

### Writing a set of textures into an APK file

```rust
let mut apk = ApkWriter::setup("path/to/archive.apk")?;
apk.add_external_file("texture1.dds")?;
apk.add_external_file_with_compression(CompressionType::ZStandard, "texture2.dds")?;
apk.save()?;
```

## APK Pack Examples

### Extracting files from an APK file

```
./metaphor-apk-pack.exe [input APK] (output folder)
```
(`./metaphor-apk-pack` on Linux). You can also drag an APK onto the executable to extract it.

Where
- **Input APK**: The target APK to extract files from
- **Output Folder** (optional): Where the folder containing extracted files will be located. By default, this will be in the same directory as the Input APK.

### Repacking DDS files + FileList.txt into an APK
```
./metaphor-apk-pack.exe [input folder] (compression) (output)
```
You can also drag a folder to repack it using default compression.

Where
- **Input Folder**: Folder containing textures to repack and a `FileList.txt` to enforce file order within the archive
- **Compression** (optional): Define the compression algorithm used. Valid options are Zlib, LZ4 and ZStd. LZ4 is used if this argument is omitted.
- **Output**: The name and path of the output APK. By default, this will be in the same directory and have the same name as the input folder.

## Credits

This is based off the work of **DeathChaos** ([Github](https://github.com/DeathChaos25/), [Bluesky](https://bsky.app/profile/deathchaos.bsky.social)) 
on [MetaphorAPKPack](https://github.com/DeathChaos25/MetaphorAPKPack) (GPL-3.0).