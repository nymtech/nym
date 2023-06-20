            X13 = mem256[&w + 104]
            W10 = mem128[&w + 80],0

            4x X13right1 = X13 unsigned>> 1
            4x X13left63 = X13 << 63
            X13sigma0 = X13right1 ^ X13left63
            4x X13right8 = X13 unsigned>> 8
            X13sigma0 = X13sigma0 ^ X13right8
                2x,0 W10right19 = W10 unsigned>> 19
            4x X13left56 = X13 << 56
                2x,0 W10left45 = W10 << 45
            X13sigma0 = X13sigma0 ^ X13left56
                1x,0 W10sigma1 = W10right19 ^ W10left45
            4x X13right7 = X13 unsigned>> 7
                2x,0 W10right61 = W10 unsigned>> 61
            X13sigma0 = X13sigma0 ^ X13right7
                1x,0 W10sigma1 ^= W10right61
            4x X12 = X12 + X13sigma0
                2x,0 W10left3 = W10 << 3
            4x X12 = X12 + mem256[&w + 40]
                1x,0 W10sigma1 ^= W10left3
                2x,0 W10right6 = W10 unsigned>> 6
                1x,0 W10sigma1 ^= W10right6
            4x X12 = W10sigma1 + X12

            2x,0 W12right19 = X12 unsigned>> 19
            2x,0 W12left45 = X12 << 45
            1x,0 W12sigma1 = W12right19 ^ W12left45
            2x,0 W12right61 = X12 unsigned>> 61
            1x,0 W12sigma1 ^= W12right61
            2x,0 W12left3 = X12 << 3
            1x,0 W12sigma1 ^= W12left3
            2x,0 W12right6 = X12 unsigned>> 6
            1x,0 W12sigma1 ^= W12right6
            W12sigma1 = W12sigma1[1],W12sigma1[0]

            4x X12 = X12 + W12sigma1
            mem256[&w + 96] = X12
            4x D12 = X12 + mem256[constants + 96]
            wc12131415 = D12

