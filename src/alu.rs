pub struct MMU;

macro_rules! _inc(
    ($cpu:ident, $reg:ident) => ({
        $cpu._r.$reg += 1;

        $cpu._r.f |= ($cpu._r.f & CARRY)
        if $cpu._r.$reg == 0x00 {
            $cpu._r.f |= ZERO;
        }
        if $cpu._r.$reg & 0x0F {
            $cpu._r.f |= HALF;
        }
    })
);

macro_rules! _add(
    ($cpu:ident, $reg1:ident, $reg2:ident) => ({
        $cpu._r.$reg1 = $cpu._r.$reg2;
        $cpu._clock.m += 1;
        $cpu._clock.t += 4;
    })
);
