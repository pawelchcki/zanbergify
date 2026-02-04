"""Detailed poster effect using CLAHE contrast enhancement."""

import cv2
import numpy as np
from rembg import remove
from PIL import Image
import io

# Default colors in BGR format (Blue, Green, Red)
DEFAULT_COLOR_BG = [0, 0, 0]           # Black (Background & Shadows)
DEFAULT_COLOR_MIDTONE = [147, 20, 255]  # Deep Magenta/Pink
DEFAULT_COLOR_HIGHLIGHT = [0, 215, 255] # Bright Yellow

# Default thresholds (0-255)
DEFAULT_THRESH_LOW = 80
DEFAULT_THRESH_HIGH = 160

# Default CLAHE parameters
DEFAULT_CLIP_LIMIT = 3.0
DEFAULT_TILE_SIZE = 8


def load_and_prepare_image_detailed(
    input_path: str,
    clip_limit: float = DEFAULT_CLIP_LIMIT,
    tile_size: int = DEFAULT_TILE_SIZE,
) -> tuple[np.ndarray, np.ndarray, np.ndarray] | None:
    """Load image, remove background, apply CLAHE enhancement, and prepare for posterization.

    Returns tuple of (enhanced_gray, alpha, img_bgr) or None if failed.
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

    # Convert to grayscale
    gray = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2GRAY)

    # Apply CLAHE for local contrast enhancement
    print("  Applying CLAHE contrast enhancement...")
    clahe = cv2.createCLAHE(clipLimit=clip_limit, tileGridSize=(tile_size, tile_size))
    enhanced_gray = clahe.apply(gray)

    # Apply sharpening kernel for crisp edges
    kernel = np.array([[0, -1, 0],
                       [-1, 5, -1],
                       [0, -1, 0]])
    enhanced_gray = cv2.filter2D(enhanced_gray, -1, kernel)

    return enhanced_gray, alpha, img_bgr


def apply_detailed_posterize(
    enhanced_gray: np.ndarray,
    alpha: np.ndarray,
    img_bgr: np.ndarray,
    output_path: str,
    thresh_low: int = DEFAULT_THRESH_LOW,
    thresh_high: int = DEFAULT_THRESH_HIGH,
    color_bg: list[int] = None,
    color_midtone: list[int] = None,
    color_highlight: list[int] = None,
):
    """Apply detailed posterization effect with given thresholds and save."""
    if color_bg is None:
        color_bg = DEFAULT_COLOR_BG
    if color_midtone is None:
        color_midtone = DEFAULT_COLOR_MIDTONE
    if color_highlight is None:
        color_highlight = DEFAULT_COLOR_HIGHLIGHT

    final_img = np.zeros_like(img_bgr)

    mask_shadow = enhanced_gray < thresh_low
    mask_mid = (enhanced_gray >= thresh_low) & (enhanced_gray < thresh_high)
    mask_high = enhanced_gray >= thresh_high
    mask_subject = alpha > 10

    final_img[mask_shadow & mask_subject] = color_bg
    final_img[mask_mid & mask_subject] = color_midtone
    final_img[mask_high & mask_subject] = color_highlight

    cv2.imwrite(output_path, final_img)


def create_detailed_poster(
    input_path: str,
    output_path: str,
    thresh_low: int = DEFAULT_THRESH_LOW,
    thresh_high: int = DEFAULT_THRESH_HIGH,
    clip_limit: float = DEFAULT_CLIP_LIMIT,
    tile_size: int = DEFAULT_TILE_SIZE,
):
    """Create a detailed poster effect from an image file."""
    print(f"Processing: {input_path}...")

    result = load_and_prepare_image_detailed(input_path, clip_limit, tile_size)
    if result is None:
        return

    enhanced_gray, alpha, img_bgr = result
    apply_detailed_posterize(enhanced_gray, alpha, img_bgr, output_path, thresh_low, thresh_high)
    print(f"Success! Saved to: {output_path}")


def main():
    """CLI entry point for detailed poster command."""
    import sys
    if len(sys.argv) < 2:
        print("Usage: detailed <input_image> [output_image]")
        print("Example: detailed profile.jpg result.png")
        sys.exit(1)

    input_file = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else "detailed_result.png"
    create_detailed_poster(input_file, output_file)


if __name__ == "__main__":
    main()
