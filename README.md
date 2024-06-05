# rolling_stats
Statistic_lib solution for assignment below.

## Assignment
Implement a struct RollingStats for computing various statistics over a window of the most recent values:
Initialize with a nonzero window size and an endianness type (either little or big).
Accept i32 numbers in the form of a sequence of bytes in the specified endianness.
The struct implements the std::io::Write trait when the library is build with std feature flag.
At any point in time, it provides methods for computing:
Arithmetic mean (f32)
Standard deviation (f32)
A sample value from the standard distribution with the mean and standard deviation from the above points (f32)
The statistics are computed from the most recent values up to the window size. If there are fewer numbers, then all available values are used. Numbers
older than the window size should be discarded.
Write the code to be panic-free and/or briefly comment on why a potentially panicking expression can’t panic in your code.
Focus on performance and efficiency but do not sacrifice readability or safety.
Add tests to your code.
You are allowed to use any dependency from crates.io: Rust Package Registry .
Bonus: It should correctly handle "incomplete" numbers, which are completed in the next write call. For example, in sequence 1, 2, 3, the number 2
might be split such that the first two bytes are trailing in the first write call and the last two bytes are at the beginning of the next write call.
Implement this in the form of a library and publish it as a Git repository. Make the library usable in no std environment.
