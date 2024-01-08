import SwiftUI
import Theme

public struct StatusInfoView: View {
    private let timer = Timer.publish(every: 1, on: .main, in: .common).autoconnect()
    @State private var timeConnected = "00:00:00"
    @State private var startDate = Date.now

    public init() {}

    public var body: some View {
        // TODO: missing states
        Text("Initializing client...")
            .foregroundStyle(NymColor.statusInfoText)
            .textStyle(.Label.Large.primary)
        Spacer()
            .frame(height: 8)
        Text("\(timeConnected)")
            .foregroundStyle(NymColor.statusTimer)
            .textStyle(.Label.Large.primary)
            .onReceive(timer) { _ in
                timeConnected = differenceBetweenDates(startDate: startDate, currentDate: Date.now)
            }
    }
}

extension StatusInfoView {
    // TODO: move to separate date formatter service
    func differenceBetweenDates(startDate: Date, currentDate: Date) -> String {
        let formatter = DateComponentsFormatter()
        formatter.allowedUnits = [.hour, .minute, .second]
        formatter.zeroFormattingBehavior = .pad
        return formatter.string(from: startDate, to: currentDate) ?? "00:00:00"
    }
}
