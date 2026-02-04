"""Main module with example functionality."""


def greet(name: str) -> str:
    """Return a greeting message."""
    return f"Hello, {name}! Welcome to Zanbergify."


def calculate_sum(numbers: list[int]) -> int:
    """Calculate the sum of a list of numbers."""
    return sum(numbers)


def main():
    """Main entry point for the application."""
    print("Zanbergify Example Script")
    print("=" * 30)

    # Test greeting
    message = greet("World")
    print(message)

    # Test calculation
    numbers = [1, 2, 3, 4, 5]
    total = calculate_sum(numbers)
    print(f"Sum of {numbers} = {total}")

    print("=" * 30)
    print("Script completed successfully!")


if __name__ == "__main__":
    main()
