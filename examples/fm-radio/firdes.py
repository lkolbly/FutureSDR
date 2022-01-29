# From https://fiiir.com/

import numpy as np

# Example code, computes the coefficients of a low-pass windowed-sinc filter.

# Configuration.
fS = 8000000  # Sampling rate.
fL = 80000  # Cutoff frequency.
N = 59  # Filter length, must be odd.

# Compute sinc filter.
h = np.sinc(2 * fL / fS * (np.arange(N) - (N - 1) / 2))

# Apply window.
h *= np.blackman(N)

# Normalize to get unity gain.
h /= np.sum(h)

print(",".join(list(map(lambda x: str(x), h))))

#
# Filter 2
#

# Configuration.
fS = 800000  # Sampling rate.
fL = 72000  # Cutoff frequency.
N = 115  # Filter length, must be odd.

# Compute sinc filter.
h = np.sinc(2 * fL / fS * (np.arange(N) - (N - 1) / 2))

# Apply window.
h *= np.blackman(N)

# Normalize to get unity gain.
h /= np.sum(h)

print(",".join(list(map(lambda x: str(x), h))))
