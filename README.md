# dylibtree

`dylibtree` is a tool for inspecting the dynamic dependencies of a
Mach-O binary recursively. It can be useful to understand what library
loads another library that you may not expect. If it helps you can think
of `dylibtree` as a recursive `otool -L`, or as a Mach-O version of `lddtree`.

# Usage

To list all recursive dynamic dependencies, just pass a binary:

```
$ dylibtree /usr/bin/xcrun
/usr/bin/xcrun:
  /usr/lib/libxcselect.dylib:
    /usr/lib/libSystem.B.dylib:
      /usr/lib/system/libcache.dylib:
        /usr/lib/system/libsystem_malloc.dylib:
          /usr/lib/system/libcompiler_rt.dylib:
            /usr/lib/system/libunwind.dylib:
              /usr/lib/system/libsystem_malloc.dylib
...
```

You can limit the depth (and in turn the output) with `--depth N`:

```
$ dylibtree /usr/bin/xcrun --depth 2
/usr/bin/xcrun:
  /usr/lib/libxcselect.dylib:
    /usr/lib/libSystem.B.dylib:
  /usr/lib/libSystem.B.dylib
```

You can also exclude various prefixes depending on what you're
investigating with `--ignore-prefix`:

```
$ dylibtree /Applications/Xcode.app/Contents/Applications/Instruments.app/Contents/MacOS/Instruments --ignore-prefix /usr/lib --ignore-prefix /System/Library
/Applications/Xcode.app/Contents/Applications/Instruments.app/Contents/MacOS/Instruments:
  @rpath/DVTInstrumentsUtilities.framework/Versions/A/DVTInstrumentsUtilities:
  @rpath/DVTInstrumentsFoundation.framework/Versions/A/DVTInstrumentsFoundation:
    @rpath/CoreSymbolicationDT.framework/Versions/A/CoreSymbolicationDT:
...
```

# Installation

```
brew install keith/formulae/dylibtree
```

## Implementation notes

- `dylibtree` uses the dyld shared cache, or other platform's runtime
  roots to discover dylibs. If you want to run `dylibtree` on a binary
  for a non macOS platform you must have that platform installed in
  Xcode, and must have built to a device with that platform to
  populate the symbols.
- `dylibtree` looks up the current locations for the runtime root for a
  platform, that can change over time, or you might download one
  manually that you want to use instead. If so you can pass
  `--runtime-root` or `--shared-cache-path` to override the default
  discovery. If the location has changed please submit an issue or PR
  so we can update it for everyone.
- dyld shared cache extraction uses Xcode's internal library, which
  means your currently selected Xcode version must support any shared
  caches being extracted. If you're on a beta version you'll likely need
  to have a beta Xcode selected (or set with `DEVELOPER_DIR`).
