#!/usr/bin/env python3
"""
rtt_histogram.py

Usage:
    python rtt_histogram.py <csv_path> <outlier_mode>

Examples:
    python rtt_histogram.py rtt_stats.csv all
        -> one histogram with ALL avg_rtt values

    python rtt_histogram.py rtt_stats.csv 1.0
        -> one histogram only for avg_rtt <= 1.0s (filters out values above 1 second)

    python rtt_histogram.py rtt_stats.csv both:1.0
        -> TWO histograms:
           1) inliers:  avg_rtt <= 1.0s
           2) outliers: avg_rtt > 1.0s
"""

import csv
import sys
import statistics
import matplotlib.pyplot as plt


def load_rtt_values(csv_path: str) -> list[float]:
    """
    Read the CSV file and return a list of RTT values in milliseconds (float).

    It expects a column named 'avg_rtt' (as written by write_csv).
    If 'avg_rtt' is not found, it falls back to 'rtt_ms'.
    """
    values_ms: list[float] = []

    with open(csv_path, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        fieldnames = [name.strip() for name in (reader.fieldnames or [])]

        if "avg_rtt" in fieldnames:
            col = "avg_rtt"
        elif "rtt_ms" in fieldnames:
            col = "rtt_ms"
        else:
            raise RuntimeError(
                f"CSV '{csv_path}' does not contain 'avg_rtt' or 'rtt_ms' column. "
                f"Found columns: {fieldnames}"
            )

        for row in reader:
            raw = row.get(col, "").strip()
            if not raw:
                continue
            try:
                values_ms.append(float(raw))
            except ValueError:
                # Ignore rows where the RTT column is not a valid number
                continue

    return values_ms


def plot_hist(values_sec, title_suffix: str, output_suffix: str | None = None):
    """
    Plot a single histogram for the given RTT values (in seconds).

    If output_suffix is provided, it can be used to build a filename
    with plt.savefig, if you want. Right now we only show the figure.
    """
    if not values_sec:
        print(f"No RTT values for plot '{title_suffix}', skipping.")
        return

    median_val = statistics.median(values_sec)

    plt.figure(figsize=(8, 6))
    plt.hist(values_sec, bins=30, edgecolor="black")
    plt.axvline(
        median_val,
        color="red",
        linestyle="--",
        linewidth=2,
        label=f"Median: {median_val:.2f}s",
    )

    plt.title(f"Distribution of RTTs {title_suffix}")
    plt.xlabel("RTT (seconds)")
    plt.ylabel("Frequency")
    plt.legend()
    plt.tight_layout()
    # If you want to save instead of show, uncomment the following
    # if output_suffix is not None:
    #     filename = f"rtt_hist_{output_suffix}.png"
    #     plt.savefig(filename)
    #     print(f"Saved plot to {filename}")
    # else:
    #     plt.show()


def main():
    if len(sys.argv) != 3:
        print("Usage: python rtt_histogram.py <csv_path> <outlier_mode>", file=sys.stderr)
        print("  outlier_mode:", file=sys.stderr)
        print("    'all'           -> one plot with all RTTs", file=sys.stderr)
        print("    '<cutoff>'      -> one plot with RTT <= cutoff seconds", file=sys.stderr)
        print("    'both:<cutoff>' -> two plots: inliers & outliers around cutoff", file=sys.stderr)
        sys.exit(1)

    csv_path = sys.argv[1]
    outlier_mode = sys.argv[2].strip().lower()

    # 1) Load avg_rtt values (ms)
    try:
        rtt_ms = load_rtt_values(csv_path)
    except Exception as e:
        print(f"Error reading CSV: {e}", file=sys.stderr)
        sys.exit(1)

    if not rtt_ms:
        print("No RTT values found in CSV – nothing to plot.")
        sys.exit(0)

    # 2) Convert to seconds
    rtt_sec = [v / 1000.0 for v in rtt_ms]

    # --------------------------------------------------------------------
    # MODE 1: both:<cutoff>  → generate TWO plots (inliers & outliers)
    # --------------------------------------------------------------------
    if outlier_mode.startswith("both:"):
        cutoff_str = outlier_mode.split(":", 1)[1]
        try:
            cutoff = float(cutoff_str)
        except ValueError:
            print(
                f"Invalid outlier_mode '{outlier_mode}'. "
                f"Expected format 'both:<numeric_cutoff>', e.g. 'both:1.0'.",
                file=sys.stderr,
            )
            sys.exit(1)

        inliers = [v for v in rtt_sec if v <= cutoff]
        outliers = [v for v in rtt_sec if v > cutoff]

        if not inliers and not outliers:
            print("No RTT values found for inliers or outliers – nothing to plot.")
            sys.exit(0)

        print(f"Total RTT samples: {len(rtt_sec)}")
        print(f"Inliers (<= {cutoff:.3f}s): {len(inliers)}")
        print(f"Outliers (>  {cutoff:.3f}s): {len(outliers)}")

        # Plot inliers
        plot_hist(inliers, title_suffix=f"(inliers ≤ {cutoff:.2f}s)", output_suffix="inliers")
        # Plot outliers
        plot_hist(outliers, title_suffix=f"(outliers > {cutoff:.2f}s)", output_suffix="outliers")

        # Show all open figures
        plt.show()
        sys.exit(0)

    # --------------------------------------------------------------------
    # MODE 2: all  → single plot with all RTT values
    # --------------------------------------------------------------------
    if outlier_mode == "all":
        if not rtt_sec:
            print("No RTT values to plot.")
            sys.exit(0)

        plot_hist(rtt_sec, title_suffix="(all samples)", output_suffix=None)
        plt.show()
        sys.exit(0)

    # --------------------------------------------------------------------
    # MODE 3: numeric cutoff  → single plot with inliers only
    # --------------------------------------------------------------------
    try:
        cutoff = float(outlier_mode)
    except ValueError:
        print(
            f"Invalid outlier_mode '{outlier_mode}'. "
            f"Use 'all', a numeric cutoff (e.g. '1.0'), or 'both:<cutoff>'.",
            file=sys.stderr,
        )
        sys.exit(1)

    filtered = [v for v in rtt_sec if v <= cutoff]

    if not filtered:
        print(
            f"After filtering with cutoff={cutoff:.2f}s, no RTT values remain – nothing to plot."
        )
        sys.exit(0)

    print(f"Total RTT samples: {len(rtt_sec)}")
    print(f"Filtered inliers (<= {cutoff:.3f}s): {len(filtered)}")

    plot_hist(filtered, title_suffix=f"(≤ {cutoff:.2f}s)", output_suffix=None)
    plt.show()


if __name__ == "__main__":
    main()
