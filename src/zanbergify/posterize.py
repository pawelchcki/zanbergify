import cv2
import numpy as np
from rembg import remove
from PIL import Image
import io

def create_posterized_portrait(input_path, output_path):
    print(f"Processing: {input_path}...")

    # --- CONFIGURATION: COLORS & THRESHOLDS ---
    # Colors in BGR format (Blue, Green, Red)
    # These match your reference image style:
    COLOR_BG       = [0, 0, 0]        # Black (Background & Shadows)
    COLOR_MIDTONE  = [147, 20, 255]   # Deep Magenta/Pink
    COLOR_HIGHLIGHT= [0, 215, 255]    # Bright Yellow

    # Thresholds (0-255)
    # Adjust these if the image is too dark or too light
    # Pixels darker than 90 become Black
    # Pixels between 90 and 165 become Pink
    # Pixels brighter than 165 become Yellow
    THRESH_LOW = 90
    THRESH_HIGH = 165
    # -------------------------------------------

    # 1. LOAD IMAGE & REMOVE BACKGROUND
    try:
        with open(input_path, 'rb') as i:
            input_data = i.read()
            # rembg removes the background and returns an image with an Alpha channel
            output_data = remove(input_data)
            pil_img = Image.open(io.BytesIO(output_data))
    except FileNotFoundError:
        print(f"Error: Could not find file '{input_path}'")
        return

    # 2. CONVERT TO OPENCV FORMAT
    # Convert PIL image to numpy array (OpenCV format)
    # Rembg returns RGBA, so we convert to BGR for processing and keep Alpha for masking
    img_rgba = np.array(pil_img)

    # Extract the alpha channel (transparency)
    alpha = img_rgba[:, :, 3]

    # Convert the RGB part to BGR (standard OpenCV format)
    img_bgr = cv2.cvtColor(img_rgba, cv2.COLOR_RGBA2BGR)

    # 3. PRE-PROCESSING
    # Convert to grayscale to find brightness levels
    gray = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2GRAY)

    # Apply Gaussian Blur to smooth skin textures and reduce noise
    # This creates the "cartoon/poster" look rather than a pixelated look
    gray_blurred = cv2.GaussianBlur(gray, (9, 9), 0)

    # 4. CREATE MASKS
    # Create a blank canvas filled with the Background color (Black)
    final_img = np.full_like(img_bgr, COLOR_BG)

    # Logic:
    # 1. Identify where the face actually is (where alpha > 0)
    # 2. Inside the face area, apply the colors based on brightness

    # Mask for Midtones (Pink)
    mask_mid = (gray_blurred >= THRESH_LOW) & (gray_blurred < THRESH_HIGH)

    # Mask for Highlights (Yellow)
    mask_high = (gray_blurred >= THRESH_HIGH)

    # Mask for the actual subject (exclude transparent background)
    mask_subject = alpha > 10

    # 5. PAINT THE IMAGE
    # Apply Midtones where the mask matches AND it is part of the subject
    final_img[mask_mid & mask_subject] = COLOR_MIDTONE

    # Apply Highlights where the mask matches AND it is part of the subject
    final_img[mask_high & mask_subject] = COLOR_HIGHLIGHT

    # (Shadows remain the background color because we initialized the canvas that way)

    # 6. SAVE RESULT
    cv2.imwrite(output_path, final_img)
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
