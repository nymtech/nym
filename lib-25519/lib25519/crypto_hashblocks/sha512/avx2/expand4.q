            X5 = mem256[&w + 40]
            W2 = mem128[&w + 16],0

            4x X5right1 = X5 unsigned>> 1
            4x X5left63 = X5 << 63
            X5sigma0 = X5right1 ^ X5left63
            4x X5right8 = X5 unsigned>> 8
            X5sigma0 = X5sigma0 ^ X5right8
                2x,0 W2right19 = W2 unsigned>> 19
            4x X5left56 = X5 << 56
                2x,0 W2left45 = W2 << 45
            X5sigma0 = X5sigma0 ^ X5left56
                1x,0 W2sigma1 = W2right19 ^ W2left45
            4x X5right7 = X5 unsigned>> 7
                2x,0 W2right61 = W2 unsigned>> 61
            X5sigma0 = X5sigma0 ^ X5right7
                1x,0 W2sigma1 ^= W2right61
            4x X4 = X4 + X5sigma0
                2x,0 W2left3 = W2 << 3
            4x X4 = X4 + mem256[&w + 104]
                1x,0 W2sigma1 ^= W2left3
                2x,0 W2right6 = W2 unsigned>> 6
                1x,0 W2sigma1 ^= W2right6
            4x X4 = W2sigma1 + X4

            2x,0 W4right19 = X4 unsigned>> 19
            2x,0 W4left45 = X4 << 45
            1x,0 W4sigma1 = W4right19 ^ W4left45
            2x,0 W4right61 = X4 unsigned>> 61
            1x,0 W4sigma1 ^= W4right61
            2x,0 W4left3 = X4 << 3
            1x,0 W4sigma1 ^= W4left3
            2x,0 W4right6 = X4 unsigned>> 6
            1x,0 W4sigma1 ^= W4right6
            W4sigma1 = W4sigma1[1],W4sigma1[0]

            4x X4 = X4 + W4sigma1
            mem256[&w + 32] = X4
            4x D4 = X4 + mem256[constants + 32]
            wc4567 = D4

