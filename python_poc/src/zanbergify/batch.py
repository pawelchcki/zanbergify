"""Batch processing script for posterizing images in work_folder."""

import sys
from pathlib import Path

from zanbergify.posterize import load_and_prepare_image, apply_posterize
from zanbergify.painted import load_and_prepare_image_painted, apply_painted_posterize
from zanbergify.detailed import load_and_prepare_image_detailed, apply_detailed_posterize

# Supported image extensions
IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp", ".bmp"}

# Posterize threshold presets: (name, thresh_low, thresh_high)
POSTERIZE_PRESETS = [
    ("dark", 60, 130),      # More shadows, dramatic look
    ("balanced", 90, 165),  # Default balanced look
    ("bright", 120, 190),   # More highlights, lighter look
    ("contrast", 70, 180),  # High contrast, less midtones
    ("soft", 100, 150),     # More midtones, softer look
]

# Painted presets: (name, thresh_low, thresh_high, mean_shift_sp, mean_shift_sr)
PAINTED_PRESETS = [
    ("painted_smooth", 90, 165, 25, 50),   # Smooth painted look
    ("painted_detail", 90, 165, 15, 35),   # More detail preserved
    ("painted_abstract", 90, 165, 35, 60), # More abstract/flat
]

# Detailed presets: (name, thresh_low, thresh_high, clip_limit, tile_size)
DETAILED_PRESETS = [
    ("detailed_standard", 80, 160, 3.0, 8),   # Standard CLAHE enhancement
    ("detailed_strong", 70, 150, 4.0, 8),     # Stronger contrast enhancement
    ("detailed_fine", 80, 160, 2.5, 4),       # Finer detail preservation
]

# All preset names for filtering generated files
ALL_PRESET_NAMES = (
    [p[0] for p in POSTERIZE_PRESETS] +
    [p[0] for p in PAINTED_PRESETS] +
    [p[0] for p in DETAILED_PRESETS]
)


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
    # Check for preset suffixes
    if any(stem.endswith(f"_{name}") for name in ALL_PRESET_NAMES):
        return True
    # Check for old _posterized suffix
    if stem.endswith("_posterized"):
        return True
    return False


def process_batch(work_folder: Path | None = None, output_folder: Path | None = None):
    """Process all images in work_folder with multiple algorithms and presets."""
    if work_folder is None:
        project_root = Path(__file__).parent.parent.parent
        work_folder = project_root / "work_folder"

    if output_folder is None:
        output_folder = work_folder / "output"

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

    total_presets = len(POSTERIZE_PRESETS) + len(PAINTED_PRESETS) + len(DETAILED_PRESETS)
    processed = 0
    skipped = 0

    for input_path in image_files:
        # === POSTERIZE ALGORITHM ===
        posterize_to_process = []
        for preset_name, thresh_low, thresh_high in POSTERIZE_PRESETS:
            output_path = get_output_path(input_path, output_folder, preset_name)
            if needs_processing(input_path, output_path, source_mtime):
                posterize_to_process.append((preset_name, thresh_low, thresh_high, output_path))

        if posterize_to_process:
            print(f"\n[Posterize] Loading: {input_path.name}")
            result = load_and_prepare_image(str(input_path))
            if result is not None:
                gray_blurred, alpha, img_bgr = result
                for preset_name, thresh_low, thresh_high, output_path in posterize_to_process:
                    print(f"  Generating {preset_name} (low={thresh_low}, high={thresh_high})...")
                    apply_posterize(gray_blurred, alpha, img_bgr, str(output_path), thresh_low, thresh_high)
                    processed += 1
        else:
            print(f"[Posterize] Skipping (up to date): {input_path.name}")

        skipped += len(POSTERIZE_PRESETS) - len(posterize_to_process)

        # === PAINTED ALGORITHM ===
        painted_to_process = []
        for preset_name, thresh_low, thresh_high, sp, sr in PAINTED_PRESETS:
            output_path = get_output_path(input_path, output_folder, preset_name)
            if needs_processing(input_path, output_path, source_mtime):
                painted_to_process.append((preset_name, thresh_low, thresh_high, sp, sr, output_path))

        if painted_to_process:
            # Group by mean shift params to avoid redundant processing
            by_params: dict[tuple[int, int], list] = {}
            for preset_name, thresh_low, thresh_high, sp, sr, output_path in painted_to_process:
                key = (sp, sr)
                if key not in by_params:
                    by_params[key] = []
                by_params[key].append((preset_name, thresh_low, thresh_high, output_path))

            for (sp, sr), presets in by_params.items():
                print(f"\n[Painted] Loading: {input_path.name} (sp={sp}, sr={sr})")
                result = load_and_prepare_image_painted(str(input_path), sp, sr)
                if result is not None:
                    gray, alpha_eroded, img_bgr = result
                    for preset_name, thresh_low, thresh_high, output_path in presets:
                        print(f"  Generating {preset_name} (low={thresh_low}, high={thresh_high})...")
                        apply_painted_posterize(gray, alpha_eroded, img_bgr, str(output_path), thresh_low, thresh_high)
                        processed += 1
        else:
            print(f"[Painted] Skipping (up to date): {input_path.name}")

        skipped += len(PAINTED_PRESETS) - len(painted_to_process)

        # === DETAILED ALGORITHM ===
        detailed_to_process = []
        for preset_name, thresh_low, thresh_high, clip_limit, tile_size in DETAILED_PRESETS:
            output_path = get_output_path(input_path, output_folder, preset_name)
            if needs_processing(input_path, output_path, source_mtime):
                detailed_to_process.append((preset_name, thresh_low, thresh_high, clip_limit, tile_size, output_path))

        if detailed_to_process:
            # Group by CLAHE params to avoid redundant processing
            by_params: dict[tuple[float, int], list] = {}
            for preset_name, thresh_low, thresh_high, clip_limit, tile_size, output_path in detailed_to_process:
                key = (clip_limit, tile_size)
                if key not in by_params:
                    by_params[key] = []
                by_params[key].append((preset_name, thresh_low, thresh_high, output_path))

            for (clip_limit, tile_size), presets in by_params.items():
                print(f"\n[Detailed] Loading: {input_path.name} (clip={clip_limit}, tile={tile_size})")
                result = load_and_prepare_image_detailed(str(input_path), clip_limit, tile_size)
                if result is not None:
                    enhanced_gray, alpha, img_bgr = result
                    for preset_name, thresh_low, thresh_high, output_path in presets:
                        print(f"  Generating {preset_name} (low={thresh_low}, high={thresh_high})...")
                        apply_detailed_posterize(enhanced_gray, alpha, img_bgr, str(output_path), thresh_low, thresh_high)
                        processed += 1
        else:
            print(f"[Detailed] Skipping (up to date): {input_path.name}")

        skipped += len(DETAILED_PRESETS) - len(detailed_to_process)

    print(f"\nDone! Generated: {processed}, Skipped: {skipped}")
    print(f"Posterize presets: {', '.join(p[0] for p in POSTERIZE_PRESETS)}")
    print(f"Painted presets: {', '.join(p[0] for p in PAINTED_PRESETS)}")
    print(f"Detailed presets: {', '.join(p[0] for p in DETAILED_PRESETS)}")


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
