            X9 = mem256[&w + 72]
            W6 = mem128[&w + 48],0

            4x X9right1 = X9 unsigned>> 1
            4x X9left63 = X9 << 63
            X9sigma0 = X9right1 ^ X9left63
            4x X9right8 = X9 unsigned>> 8
            X9sigma0 = X9sigma0 ^ X9right8
                2x,0 W6right19 = W6 unsigned>> 19
            4x X9left56 = X9 << 56
                2x,0 W6left45 = W6 << 45
            X9sigma0 = X9sigma0 ^ X9left56
                1x,0 W6sigma1 = W6right19 ^ W6left45
            4x X9right7 = X9 unsigned>> 7
                2x,0 W6right61 = W6 unsigned>> 61
            X9sigma0 = X9sigma0 ^ X9right7
                1x,0 W6sigma1 ^= W6right61
            4x X8 = X8 + X9sigma0
                2x,0 W6left3 = W6 << 3
            4x X8 = X8 + mem256[&w + 8]
                1x,0 W6sigma1 ^= W6left3
                2x,0 W6right6 = W6 unsigned>> 6
                1x,0 W6sigma1 ^= W6right6
            4x X8 = W6sigma1 + X8

            2x,0 W8right19 = X8 unsigned>> 19
            2x,0 W8left45 = X8 << 45
            1x,0 W8sigma1 = W8right19 ^ W8left45
            2x,0 W8right61 = X8 unsigned>> 61
            1x,0 W8sigma1 ^= W8right61
            2x,0 W8left3 = X8 << 3
            1x,0 W8sigma1 ^= W8left3
            2x,0 W8right6 = X8 unsigned>> 6
            1x,0 W8sigma1 ^= W8right6
            W8sigma1 = W8sigma1[1],W8sigma1[0]

            4x X8 = X8 + W8sigma1
            mem256[&w + 64] = X8
            4x D8 = X8 + mem256[constants + 64]
            wc891011 = D8

