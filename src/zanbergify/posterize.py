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
DEFAULT_THRESH_LOW = 90
DEFAULT_THRESH_HIGH = 165


def load_and_prepare_image(input_path: str) -> tuple[np.ndarray, np.ndarray, np.ndarray] | None:
    """Load image, remove background, and prepare for posterization.

    Returns tuple of (gray_blurred, alpha, img_bgr) or None if failed.
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
    gray = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2GRAY)
    gray_blurred = cv2.GaussianBlur(gray, (9, 9), 0)

    return gray_blurred, alpha, img_bgr


def apply_posterize(
    gray_blurred: np.ndarray,
    alpha: np.ndarray,
    img_bgr: np.ndarray,
    output_path: str,
    thresh_low: int = DEFAULT_THRESH_LOW,
    thresh_high: int = DEFAULT_THRESH_HIGH,
    color_bg: list[int] = None,
    color_midtone: list[int] = None,
    color_highlight: list[int] = None,
):
    """Apply posterization effect with given thresholds and save."""
    if color_bg is None:
        color_bg = DEFAULT_COLOR_BG
    if color_midtone is None:
        color_midtone = DEFAULT_COLOR_MIDTONE
    if color_highlight is None:
        color_highlight = DEFAULT_COLOR_HIGHLIGHT

    final_img = np.full_like(img_bgr, color_bg)

    mask_mid = (gray_blurred >= thresh_low) & (gray_blurred < thresh_high)
    mask_high = (gray_blurred >= thresh_high)
    mask_subject = alpha > 10

    final_img[mask_mid & mask_subject] = color_midtone
    final_img[mask_high & mask_subject] = color_highlight

    cv2.imwrite(output_path, final_img)


def create_posterized_portrait(
    input_path: str,
    output_path: str,
    thresh_low: int = DEFAULT_THRESH_LOW,
    thresh_high: int = DEFAULT_THRESH_HIGH,
):
    """Create a posterized portrait from an image file."""
    print(f"Processing: {input_path}...")

    result = load_and_prepare_image(input_path)
    if result is None:
        return

    gray_blurred, alpha, img_bgr = result
    apply_posterize(gray_blurred, alpha, img_bgr, output_path, thresh_low, thresh_high)
    print(f"Success! Saved to: {output_path}")

def main():
    """CLI entry point for posterize command."""
    import sys
    if len(sys.argv) < 2:
        print("Usage: posterize <input_image> [output_image]")
        print("Example: posterize profile.jpg result.png")
        sys.exit(1)

    input_file = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else "result.png"
    create_posterized_portrait(input_file, output_file)


if __name__ == "__main__":
    main()
