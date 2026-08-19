#!/usr/bin/env perl
use strict;
use warnings;
use Time::HiRes qw(clock_gettime sleep CLOCK_MONOTONIC);

my ($rate, $gate, $label) = @ARGV;
die "usage: $0 <rate-hz> <gate-file> <label>\n" unless $rate && $gate && $label;

sleep 0.01 until -e $gate;
$| = 1;
my $period = 1 / $rate;
my $next = clock_gettime(CLOCK_MONOTONIC);
my $sequence = 0;
while (1) {
    $sequence++;
    printf "\rbench-output-%08d-%s", $sequence, $label;
    $next += $period;
    my $remaining = $next - clock_gettime(CLOCK_MONOTONIC);
    sleep $remaining if $remaining > 0;
}
