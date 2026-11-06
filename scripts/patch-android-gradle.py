from pathlib import Path

path = Path("src-tauri/gen/android/app/build.gradle.kts")
if not path.exists():
    raise SystemExit(f"Android Gradle file not found: {path}")

text = path.read_text(encoding="utf-8")
namespace = 'namespace = "com.gym_manager.dc"'

if "ignoreAssetsPattern" not in text:
    if namespace not in text:
        raise SystemExit("Could not find the Android namespace in build.gradle.kts")
    text = text.replace(
        namespace,
        namespace
        + "\n    androidResources {\n"
        + '        ignoreAssetsPattern = "!.svn:!.git:!.ds_store:!*.scc:.*:!CVS:!thumbs.db:!picasa.ini:!*~"\n'
        + "    }",
        1,
    )

if "buildConfig = false" in text:
    text = text.replace("buildConfig = false", "buildConfig = true", 1)
elif "buildConfig = true" not in text:
    if "buildFeatures {" in text:
        text = text.replace(
            "buildFeatures {",
            "buildFeatures {\n        buildConfig = true",
            1,
        )
    else:
        raise SystemExit("Could not find buildFeatures in build.gradle.kts")

path.write_text(text, encoding="utf-8")
print(f"Patched {path}")
