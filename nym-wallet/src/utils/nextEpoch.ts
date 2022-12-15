import { getCurrentInterval } from 'src/requests';
import { add, format, fromUnixTime } from 'date-fns';

export const getIntervalAsDate = async () => {
    const interval = await getCurrentInterval();
    const secondsToNextInterval =
        Number(interval.epochs_in_interval - interval.current_epoch_id) * Number(interval.epoch_length_seconds);

    const intervalTime = format(
        add(new Date(), {
            seconds: secondsToNextInterval,
        }),
        'MM/dd/yyyy HH:mm',
    );
    const nextEpoch = format(
        add(fromUnixTime(Number(interval.current_epoch_start_unix)), {
            seconds: Number(interval.epoch_length_seconds),
        }),
        'HH:mm',
    );

    return { intervalTime, nextEpoch };
};