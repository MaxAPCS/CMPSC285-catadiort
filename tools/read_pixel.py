#!/usr/bin/env python3
"""Print linear RGB at a PFM pixel; X and Y count from the top left."""
import argparse
from compare import read_pfm


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image")
    parser.add_argument("x", type=int)
    parser.add_argument("y", type=int)
    args = parser.parse_args()
    image = read_pfm(args.image)
    if not (0 <= args.x < image.shape[1] and 0 <= args.y < image.shape[0]):
        parser.error("pixel lies outside the image")
    print(image[args.y, args.x])


if __name__ == "__main__":
    main()
