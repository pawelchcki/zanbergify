"""Painted poster effect using Mean Shift filtering."""

import cv2
import numpy as np
from rembg import remove
from PIL import Image
import io

# Default colors in BGR format (Blue, Green, Red)
DEFAULT_COLOR_BG = [0, 0, 0]           # Black (Background & Shadows)
DEFAULT_COLOR_MIDTONE = [147, 20, 255]  # Deep Magenta/Pink
DEFAULT_COLOR_HIGHLIGHT = [0, 215, 255] # Bright Yellow

# Default Mean Shift parameters
DEFAULT_MEAN_SHIFT_SP = 20  # Spatial window radius
DEFAULT_MEAN_SHIFT_SR = 45  # Color window radius

# Default thresholds (0-255)
DEFAULT_THRESH_LOW = 90
DEFAULT_THRESH_HIGH = 165


def load_and_prepare_image_painted(
    input_path: str,
    mean_shift_sp: int = DEFAULT_MEAN_SHIFT_SP,
    mean_shift_sr: int = DEFAULT_MEAN_SHIFT_SR,
) -> tuple[np.ndarray, np.ndarray, np.ndarray] | None:
    """Load image, remove background, apply mean shift filter, and prepare for posterization.

    Returns tuple of (gray, alpha_eroded, img_bgr) or None if failed.
    """
    try:
        with open(input_path, 'rb') as i:
            input_data = i.read()
            output_data = remove(input_data)
            pil_img = Image.open(io.BytesIO(output_data))
    except FileNotFoundError:
        print(f"Error: Could not find file '{input_path}'")
        return None

    img_rgba = np.array(pil_img)
    alpha = img_rgba[:, :, 3]
    img_bgr = cv2.cvtColor(img_rgba, cv2.COLOR_RGBA2BGR)

    # Apply Mean Shift Filter for painted look
    print("  Applying Mean Shift Filter for painted look...")
    painted_img = cv2.pyrMeanShiftFiltering(img_bgr, sp=mean_shift_sp, sr=mean_shift_sr, maxLevel=2)

    # Convert painted image to grayscale
    gray = cv2.cvtColor(painted_img, cv2.COLOR_BGR2GRAY)

    # Erode alpha mask slightly to avoid halo effect
    mask_subject = alpha > 10
    kernel = np.ones((3, 3), np.uint8)
    alpha_eroded = cv2.erode(mask_subject.astype(np.uint8), kernel, iterations=1)

    return gray, alpha_eroded, img_bgr


def apply_painted_posterize(
    gray: np.ndarray,
    alpha_eroded: np.ndarray,
    img_bgr: np.ndarray,
    output_path: str,
    thresh_low: int = DEFAULT_THRESH_LOW,
    thresh_high: int = DEFAULT_THRESH_HIGH,
    color_bg: list[int] = None,
    color_midtone: list[int] = None,
    color_highlight: list[int] = None,
):
    """Apply painted posterization effect with given thresholds and save."""
    if color_bg is None:
        color_bg = DEFAULT_COLOR_BG
    if color_midtone is None:
        color_midtone = DEFAULT_COLOR_MIDTONE
    if color_highlight is None:
        color_highlight = DEFAULT_COLOR_HIGHLIGHT

    final_img = np.full_like(img_bgr, color_bg)

    mask_mid = (gray >= thresh_low) & (gray < thresh_high)
    mask_high = (gray >= thresh_high)
    mask_subject = alpha_eroded.astype(bool)

    final_img[mask_mid & mask_subject] = color_midtone
    final_img[mask_high & mask_subject] = color_highlight

    cv2.imwrite(output_path, final_img)


def create_painted_poster(
    input_path: str,
    output_path: str,
    thresh_low: int = DEFAULT_THRESH_LOW,
    thresh_high: int = DEFAULT_THRESH_HIGH,
    mean_shift_sp: int = DEFAULT_MEAN_SHIFT_SP,
    mean_shift_sr: int = DEFAULT_MEAN_SHIFT_SR,
):
    """Create a painted poster effect from an image file."""
    print(f"Processing: {input_path}...")

    result = load_and_prepare_image_painted(input_path, mean_shift_sp, mean_shift_sr)
    if result is None:
        return

    gray, alpha_eroded, img_bgr = result
    apply_painted_posterize(gray, alpha_eroded, img_bgr, output_path, thresh_low, thresh_high)
    print(f"Success! Saved to: {output_path}")


def main():
    """CLI entry point for painted poster command."""
    import sys
    if len(sys.argv) < 2:
        print("Usage: painted <input_image> [output_image]")
        print("Example: painted profile.jpg result.png")
        sys.exit(1)

    input_file = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else "painted_result.png"
    create_painted_poster(input_file, output_file)


if __name__ == "__main__":
    main()
