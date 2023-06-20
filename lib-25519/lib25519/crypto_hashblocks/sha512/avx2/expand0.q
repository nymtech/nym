            X1 = mem256[&w + 8]
            W14 = mem128[&w + 112],0

            4x X1right1 = X1 unsigned>> 1
            4x X1left63 = X1 << 63
            X1sigma0 = X1right1 ^ X1left63
            4x X1right8 = X1 unsigned>> 8
            X1sigma0 = X1sigma0 ^ X1right8
                2x,0 W14right19 = W14 unsigned>> 19
            4x X1left56 = X1 << 56
                2x,0 W14left45 = W14 << 45
            X1sigma0 = X1sigma0 ^ X1left56
                1x,0 W14sigma1 = W14right19 ^ W14left45
            4x X1right7 = X1 unsigned>> 7
                2x,0 W14right61 = W14 unsigned>> 61
            X1sigma0 = X1sigma0 ^ X1right7
                1x,0 W14sigma1 ^= W14right61
            4x X0 = X0 + X1sigma0
                2x,0 W14left3 = W14 << 3
            4x X0 = X0 + mem256[&w + 72]
                1x,0 W14sigma1 ^= W14left3
                2x,0 W14right6 = W14 unsigned>> 6
                1x,0 W14sigma1 ^= W14right6
            4x X0 = W14sigma1 + X0

            2x,0 W0right19 = X0 unsigned>> 19
            2x,0 W0left45 = X0 << 45
            1x,0 W0sigma1 = W0right19 ^ W0left45
            2x,0 W0right61 = X0 unsigned>> 61
            1x,0 W0sigma1 ^= W0right61
            2x,0 W0left3 = X0 << 3
            1x,0 W0sigma1 ^= W0left3
            2x,0 W0right6 = X0 unsigned>> 6
            1x,0 W0sigma1 ^= W0right6
            W0sigma1 = W0sigma1[1],W0sigma1[0]

            4x X0 = X0 + W0sigma1
            mem256[&w + 128] = X0
            mem256[&w + 0] = X0
            4x D0 = X0 + mem256[constants + 0]
            wc0123 = D0

