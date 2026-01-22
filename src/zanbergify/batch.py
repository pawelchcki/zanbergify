"""Batch processing script for posterizing images in work_folder."""

import sys
from pathlib import Path

from zanbergify.posterize import load_and_prepare_image, apply_posterize

# Supported image extensions
IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp", ".bmp"}

# Threshold presets: (name, thresh_low, thresh_high)
THRESHOLD_PRESETS = [
    ("dark", 60, 130),      # More shadows, dramatic look
    ("balanced", 90, 165),  # Default balanced look
    ("bright", 120, 190),   # More highlights, lighter look
    ("contrast", 70, 180),  # High contrast, less midtones
    ("soft", 100, 150),     # More midtones, softer look
]


def get_source_files() -> list[Path]:
    """Get all Python source files in the package."""
    src_dir = Path(__file__).parent
    return list(src_dir.glob("*.py"))


def get_latest_source_mtime(source_files: list[Path]) -> float:
    """Get the most recent modification time of source files."""
    if not source_files:
        return 0
    return max(f.stat().st_mtime for f in source_files)


def needs_processing(input_path: Path, output_path: Path, source_mtime: float) -> bool:
    """Check if input file needs to be processed."""
    if not output_path.exists():
        return True

    output_mtime = output_path.stat().st_mtime
    input_mtime = input_path.stat().st_mtime

    if input_mtime > output_mtime:
        return True

    if source_mtime > output_mtime:
        return True

    return False


def get_output_path(input_path: Path, output_dir: Path, preset_name: str) -> Path:
    """Generate output path for an input file with preset name."""
    return output_dir / f"{input_path.stem}_{preset_name}.png"


def is_generated_file(path: Path) -> bool:
    """Check if file is a generated posterized image."""
    stem = path.stem
    # Check for new preset suffixes
    if any(stem.endswith(f"_{preset[0]}") for preset in THRESHOLD_PRESETS):
        return True
    # Check for old _posterized suffix
    if stem.endswith("_posterized"):
        return True
    return False


def process_batch(work_folder: Path | None = None, output_folder: Path | None = None):
    """Process all images in work_folder with multiple threshold presets."""
    if work_folder is None:
        project_root = Path(__file__).parent.parent.parent
        work_folder = project_root / "work_folder"

    if output_folder is None:
        output_folder = work_folder

    if not work_folder.exists():
        print(f"Creating work folder: {work_folder}")
        work_folder.mkdir(parents=True, exist_ok=True)
        print("Add images to work_folder/ and run again.")
        return

    source_files = get_source_files()
    source_mtime = get_latest_source_mtime(source_files)

    image_files = [
        f for f in work_folder.iterdir()
        if f.is_file()
        and f.suffix.lower() in IMAGE_EXTENSIONS
        and not is_generated_file(f)
    ]

    if not image_files:
        print(f"No image files found in {work_folder}")
        return

    output_folder.mkdir(parents=True, exist_ok=True)

    processed = 0
    skipped = 0

    for input_path in image_files:
        # Check which presets need processing
        presets_to_process = []
        for preset_name, thresh_low, thresh_high in THRESHOLD_PRESETS:
            output_path = get_output_path(input_path, output_folder, preset_name)
            if needs_processing(input_path, output_path, source_mtime):
                presets_to_process.append((preset_name, thresh_low, thresh_high, output_path))

        if not presets_to_process:
            print(f"Skipping (all up to date): {input_path.name}")
            skipped += len(THRESHOLD_PRESETS)
            continue

        # Load and prepare image once
        print(f"\nLoading: {input_path.name}")
        result = load_and_prepare_image(str(input_path))
        if result is None:
            continue

        gray_blurred, alpha, img_bgr = result

        # Generate all needed presets
        for preset_name, thresh_low, thresh_high, output_path in presets_to_process:
            print(f"  Generating {preset_name} (low={thresh_low}, high={thresh_high})...")
            apply_posterize(gray_blurred, alpha, img_bgr, str(output_path), thresh_low, thresh_high)
            processed += 1

        # Count skipped presets for this image
        skipped += len(THRESHOLD_PRESETS) - len(presets_to_process)

    print(f"\nDone! Generated: {processed}, Skipped: {skipped}")
    print(f"Presets: {', '.join(p[0] for p in THRESHOLD_PRESETS)}")


def main():
    """CLI entry point for batch processing."""
    work_folder = None
    output_folder = None

    if len(sys.argv) > 1:
        work_folder = Path(sys.argv[1])
    if len(sys.argv) > 2:
        output_folder = Path(sys.argv[2])

    process_batch(work_folder, output_folder)


if __name__ == "__main__":
    main()
